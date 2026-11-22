//! Installing a Modding-OpenMW.com list into an instance.
//!
//! Mods are extracted to `<data>/<category>/<extract_to>` (umo's layout), so the config generator's data paths,
//! `<base>\<category>\<data path>`, become the instance's packages: nested folders of the data folder. What was
//! installed is kept in `<data>/.nerevar/momw.json`, so running the install again resumes it and updates mods whose
//! files changed.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{
    actions, archive, cfg_values, fetch_generated_config, fetch_modlist, is_base_game_data_path,
    relative_data_path, DownloadInfo, MomwMod,
};
use crate::instance_data::{
    load_load_order, nerevar_dir, package_abs_path, save_load_order, BackgroundOperationPhase, LoadOrder,
    LoadOrderEntry, PackageKind, PluginEntry, ProgressEmitter, LOAD_ORDER_VERSION,
};
use crate::nexus::{NexusClient, NexusFileList};

const STATE_FILE: &str = "momw.json";
const PARALLEL_DOWNLOADS: usize = 4;
const BASE_ARCHIVES: &[&str] = &["Morrowind.bsa", "Tribunal.bsa", "Bloodmoon.bsa"];

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MomwManualMod {
    pub name: String,
    pub url: String,
    pub reason: String,
    /// Where to extract it, relative to the instance's data folder.
    pub folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MomwFailedMod {
    pub name: String,
    pub error: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MomwInstallReport {
    pub modlist: String,
    pub installed: u32,
    pub up_to_date: u32,
    /// Guide entries of the list that have nothing to install
    pub guides: u32,
    pub packages: u32,
    pub manual: Vec<MomwManualMod>,
    pub failed: Vec<MomwFailedMod>,
    pub notes: Vec<String>,
}

/// What an earlier install put into the instance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallState {
    modlist: String,
    beta: bool,
    /// mod slug -> versions of its downloads
    mods: BTreeMap<String, Vec<String>>,
    /// Lines this install added to the instance's openmw.cfg overrides, and its settings.cfg text
    cfg_lines: Vec<String>,
    settings_cfg: String,
}

fn state_path(data_dir: &Path) -> PathBuf {
    nerevar_dir(data_dir).join(STATE_FILE)
}

fn load_state(data_dir: &Path) -> InstallState {
    std::fs::read_to_string(state_path(data_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_state(data_dir: &Path, state: &InstallState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(state_path(data_dir), json).map_err(|e| format!("Failed to save the install state: {e}"))
}

/// Where a download comes from.
#[derive(Debug, Clone)]
pub struct Download {
    pub url: String,
    pub file_name: String,
    /// Changes when the mod's file changes
    pub version: String,
    pub size: Option<u64>,
    /// Free Nexus accounts: the file's link comes from the player's click on its download button
    pub click: Option<NexusClick>,
}

#[derive(Debug, Clone)]
pub struct NexusClick {
    game: String,
    mod_id: u64,
    file_id: u64,
    page: String,
}

/// How long a free account's install waits for the click on a file's download button
const CLICK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15 * 60);

enum Source {
    Download(Download),
    Manual(String),
}

fn is_guide(item: &MomwMod) -> bool {
    item.download_info.is_empty() || (item.data_paths.is_empty() && item.plugins.is_empty())
}

fn nexus_ids(url: &str) -> Option<(String, u64)> {
    // https://www.nexusmods.com/morrowind/mods/54708
    let path = url.split("nexusmods.com/").nth(1)?;
    let mut parts = path.split('/');
    let game = parts.next()?.to_string();
    if parts.next()? != "mods" {
        return None;
    }
    let id = parts.next()?.split(['?', '#']).next()?.parse().ok()?;
    Some((game, id))
}

fn gitlab_project(url: &str) -> Option<String> {
    // https://modding-openmw.gitlab.io/<project>/ or https://gitlab.com/<namespace>/<project>
    if let Some(rest) = url.split("modding-openmw.gitlab.io/").nth(1) {
        let project = rest.split('/').next().filter(|p| !p.is_empty())?;
        return Some(format!("modding-openmw/{project}"));
    }
    let rest = url.split("gitlab.com/").nth(1)?;
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty() && *p != "-").take(2).collect();
    (parts.len() == 2).then(|| parts.join("/"))
}

struct Resolver {
    nexus: Option<NexusClient>,
    premium: bool,
    /// Whether nxm:// links reach Nerevar, for free accounts
    link_handler: bool,
    file_lists: HashMap<(String, u64), NexusFileList>,
    http: reqwest::Client,
}

impl Resolver {
    async fn resolve(&mut self, item: &MomwMod, info: &DownloadInfo) -> Result<Source, String> {
        if let Some(direct) = info.direct_download.as_deref().filter(|url| !url.is_empty()) {
            if direct.contains("mega.nz") {
                return Ok(Source::Manual(format!("Download it from {direct} (Mega links can't be downloaded automatically)")));
            }
            let url = if direct.contains("dropbox.com") { direct.replace("dl=0", "dl=1") } else { direct.to_string() };
            let file_name = info
                .file_name
                .clone()
                .filter(|name| !name.is_empty())
                .or_else(|| url.split(['?', '#']).next()?.rsplit('/').next().map(str::to_string))
                .unwrap_or_else(|| format!("{}.download", item.dir));
            return Ok(Source::Download(Download { version: url.clone(), url, file_name, size: None, click: None }));
        }

        if let Some((game, mod_id)) = nexus_ids(&item.url) {
            return self.resolve_nexus(item, info, &game, mod_id).await;
        }

        if let Some(project) = gitlab_project(&item.url) {
            return self.resolve_gitlab(&project).await;
        }

        Ok(Source::Manual(format!("Download it from {}", item.url)))
    }

    async fn resolve_nexus(&mut self, item: &MomwMod, info: &DownloadInfo, game: &str, mod_id: u64) -> Result<Source, String> {
        let Some(nexus) = self.nexus.clone() else {
            return Ok(Source::Manual("Link your Nexus Mods account in Nerevar's settings".into()));
        };
        if !self.premium && !self.link_handler {
            return Ok(Source::Manual(format!(
                "Download it from {}, or turn on \"Nerevar handles Nexus downloads\" in Nerevar's settings to click \
                 through the list's mods",
                item.url
            )));
        }
        let key = (game.to_string(), mod_id);
        if !self.file_lists.contains_key(&key) {
            let list = nexus.files(game, mod_id).await?;
            self.file_lists.insert(key.clone(), list);
        }
        let list = &self.file_lists[&key];

        let file = if let Some(mut file_id) = info.nexus_file_id {
            // Pinned files stay; others follow Nexus' update chain like umo
            if !info.pinned {
                let mut seen = HashSet::new();
                while let Some(update) = list.file_updates.iter().find(|u| u.old_file_id == file_id) {
                    if !seen.insert(file_id) {
                        break;
                    }
                    file_id = update.new_file_id;
                }
            }
            list.files.iter().find(|f| f.file_id == file_id).or_else(|| {
                info.nexus_file_id.and_then(|original| list.files.iter().find(|f| f.file_id == original))
            })
        } else {
            let wanted = info.file_name.as_deref().unwrap_or_default();
            let mut candidates: Vec<_> = list
                .files
                .iter()
                .filter(|f| f.name == wanted)
                .filter(|f| !matches!(f.category_name.as_deref(), Some("OLD_VERSION") | Some("ARCHIVED")))
                .collect();
            candidates.sort_by_key(|f| f.category_id);
            candidates.into_iter().next()
        };
        let Some(file) = file else {
            return Ok(Source::Manual(format!(
                "The file \"{}\" isn't on {} anymore; download it yourself",
                info.file_name.as_deref().unwrap_or("?"),
                item.url
            )));
        };
        let file_name =
            if file.file_name.is_empty() { format!("{}.archive", file.file_id) } else { file.file_name.clone() };
        let (url, click) = if self.premium {
            (nexus.download_link(game, mod_id, file.file_id).await?, None)
        } else {
            let page = format!("https://www.nexusmods.com/{game}/mods/{mod_id}?tab=files&file_id={}&nmm=1", file.file_id);
            (String::new(), Some(NexusClick { game: game.to_string(), mod_id, file_id: file.file_id, page }))
        };
        Ok(Source::Download(Download {
            url,
            file_name,
            version: format!("nexus:{}", file.file_id),
            size: file.size_in_bytes,
            click,
        }))
    }

    async fn resolve_gitlab(&self, project: &str) -> Result<Source, String> {
        #[derive(Deserialize)]
        struct Package {
            id: u64,
        }
        #[derive(Deserialize)]
        struct PackageFile {
            id: u64,
            file_name: String,
            #[serde(default)]
            size: Option<u64>,
        }
        #[derive(Deserialize)]
        struct Commit {
            id: String,
        }
        let api = format!("https://gitlab.com/api/v4/projects/{}", urlencoding::encode(project));
        let packages: Vec<Package> = self.get_json(&format!("{api}/packages?sort=desc")).await.unwrap_or_default();
        if let Some(package) = packages.first() {
            let files: Vec<PackageFile> = self.get_json(&format!("{api}/packages/{}/package_files", package.id)).await?;
            if let Some(file) = files.first() {
                return Ok(Source::Download(Download {
                    url: format!("https://gitlab.com/{project}/-/package_files/{}/download", file.id),
                    file_name: file.file_name.clone(),
                    version: format!("gitlab:{}", file.id),
                    size: file.size,
                    click: None,
                }));
            }
        }
        // Projects without packages: the latest commit's sources
        let commits: Vec<Commit> = self.get_json(&format!("{api}/repository/commits?per_page=1")).await?;
        let commit = commits.first().ok_or_else(|| format!("{project} has no commits"))?;
        Ok(Source::Download(Download {
            url: format!("{api}/repository/archive.zip?sha={}", commit.id),
            file_name: format!("{}-{}.zip", project.replace('/', "-"), &commit.id[..commit.id.len().min(12)]),
            version: format!("gitlab-commit:{}", commit.id),
            size: None,
            click: None,
        }))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, String> {
        let response = self.http.get(url).send().await.map_err(|e| format!("{url}: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("{url} returned {}", response.status()));
        }
        response.json().await.map_err(|e| format!("Unexpected answer from {url}: {e}"))
    }
}

pub fn archive_cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("Nerevar").join("archives")
}

fn safe_file_name(name: &str) -> String {
    name.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect()
}

/// For free accounts: opens the file's page and waits for the player's click on its download button.
async fn link_from_click(
    nexus: &NexusClient,
    click: &NexusClick,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<String, String> {
    let waiting = crate::nexus::nxm::wait_for(click.mod_id, click.file_id);
    opener::open_browser(&click.page).map_err(|e| format!("Couldn't open {}: {e}", click.page))?;
    let started = std::time::Instant::now();
    let mut waiting = std::pin::pin!(waiting);
    let link = loop {
        tokio::select! {
            link = &mut waiting => break link.map_err(|_| "Stopped waiting for the download".to_string())?,
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                let timed_out = started.elapsed() > CLICK_TIMEOUT;
                if cancel.load(std::sync::atomic::Ordering::Relaxed) || timed_out {
                    crate::nexus::nxm::stop_waiting(click.mod_id, click.file_id);
                    return Err(if timed_out {
                        "Nobody clicked \"Mod Manager Download\" on the mod's page".into()
                    } else {
                        "Install cancelled".into()
                    });
                }
            }
        }
    };
    nexus
        .download_link_with_key(&click.game, click.mod_id, click.file_id, link.key.as_deref(), link.expires.as_deref())
        .await
}

pub async fn download_file(http: &reqwest::Client, download: &Download, dest: &Path) -> Result<(), String> {
    if let (Ok(metadata), Some(size)) = (std::fs::metadata(dest), download.size) {
        if metadata.len() == size {
            return Ok(()); // downloaded before
        }
    } else if download.size.is_none() && dest.is_file() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let response = http.get(&download.url).send().await.map_err(|e| format!("Download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Download returned {}", response.status()));
    }
    let part = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download interrupted: {e}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await.map_err(|e| e.to_string())?;
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await.map_err(|e| e.to_string())?;
    drop(file);
    tokio::fs::rename(&part, dest).await.map_err(|e| e.to_string())
}

/// Extracts the downloads of a mod into its category folder and runs their actions.
fn install_mod(data_dir: &Path, item: &MomwMod, archives: &[(DownloadInfo, Download, PathBuf)]) -> Result<Vec<String>, String> {
    let category_dir = data_dir.join(&item.category);
    // A fresh copy of the mod: remove what an older version left
    let mut cleared = HashSet::new();
    for (info, _, _) in archives {
        let extract_to = info.extract_to.clone().unwrap_or_else(|| item.dir.clone());
        if cleared.insert(extract_to.clone()) {
            let target = category_dir.join(&extract_to);
            if target.exists() {
                std::fs::remove_dir_all(&target).map_err(|e| format!("Failed to remove {}: {e}", target.display()))?;
            }
        }
    }
    let mut notes = Vec::new();
    for (info, download, path) in archives {
        let extract_to = info.extract_to.clone().unwrap_or_else(|| item.dir.clone());
        let target = category_dir.join(&extract_to);
        archive::extract(path, &target, &download.file_name)?;
        if download.version.starts_with("gitlab-commit:") {
            flatten_single_folder(&target)?;
        }
        for action in &info.actions {
            if let Some(note) = actions::run(&category_dir, action)? {
                notes.push(format!("{}: {note}", item.name));
            }
        }
    }
    Ok(notes)
}

/// Source archives of GitLab commits have everything in one folder; move it up like umo does.
fn flatten_single_folder(dir: &Path) -> Result<(), String> {
    let entries: Vec<_> = std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten().collect();
    if entries.len() != 1 || !entries[0].path().is_dir() {
        return Ok(());
    }
    let inner = entries[0].path();
    for entry in std::fs::read_dir(&inner).map_err(|e| e.to_string())?.flatten() {
        std::fs::rename(entry.path(), dir.join(entry.file_name())).map_err(|e| e.to_string())?;
    }
    std::fs::remove_dir(&inner).map_err(|e| e.to_string())
}

fn root_content_files(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && crate::openmw_ini_importer::is_openmw_content_path(path))
        .filter_map(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    files.sort_by_key(|name| name.to_ascii_lowercase());
    files
}

/// The instance's load order and settings from the list's generated config. Packages of other mods stay after the
/// list's.
fn apply_config(
    data_dir: &Path,
    slug: &str,
    config: &super::GeneratedConfig,
    state: &mut InstallState,
    report: &mut MomwInstallReport,
) -> Result<(), String> {
    let mut load_order = load_load_order(data_dir).unwrap_or_else(|_| LoadOrder::default());
    let content: Vec<String> = cfg_values(&config.openmw_cfg.content_files, "content").into_iter().map(str::to_string).collect();
    let wanted: HashSet<String> = content.iter().map(|c| c.to_ascii_lowercase()).collect();

    let mut list_entries = Vec::new();
    let mut seen = HashSet::new();
    let mut missing = 0;
    for path in cfg_values(&config.openmw_cfg.data_paths, "data") {
        if is_base_game_data_path(path) {
            continue;
        }
        let Some(relative) = relative_data_path(path) else {
            report.notes.push(format!("Skipped a data path outside the list's folders: {path}"));
            continue;
        };
        if !seen.insert(relative.to_ascii_lowercase()) {
            continue;
        }
        let dir = package_abs_path(data_dir, &relative);
        if !dir.is_dir() {
            missing += 1;
            continue;
        }
        let existing = load_order.entries.iter().find(|e| e.relative_dir.eq_ignore_ascii_case(&relative)).cloned();
        let plugin_files = root_content_files(&dir);
        let plugins: Vec<PluginEntry> = plugin_files
            .iter()
            .map(|file| PluginEntry { file: file.clone(), enabled: wanted.contains(&file.to_ascii_lowercase()) })
            .collect();
        let name = relative.rsplit('/').next().unwrap_or(&relative).to_string();
        list_entries.push(LoadOrderEntry {
            id: existing.as_ref().map(|e| e.id.clone()).unwrap_or_else(crate::instance_data::new_entry_id),
            name: existing.as_ref().map(|e| e.name.clone()).unwrap_or(name),
            kind: if plugins.is_empty() { PackageKind::Replacer } else { PackageKind::Mod },
            shader_kind: existing
                .as_ref()
                .and_then(|e| e.shader_kind)
                .or_else(|| crate::shaders::packages::detect_shader_kind(&dir, &plugin_files)),
            relative_dir: relative,
            enabled: true,
            priority: 0,
            plugins,
            tree_checksum: None,
        });
    }
    if missing > 0 {
        report.notes.push(format!("{missing} data folders of the list are missing until their mods are installed"));
    }

    let list_dirs: HashSet<String> = list_entries.iter().map(|e| e.relative_dir.to_ascii_lowercase()).collect();
    let others: Vec<LoadOrderEntry> = load_order
        .entries
        .drain(..)
        .filter(|e| !list_dirs.contains(&e.relative_dir.to_ascii_lowercase()))
        .collect();
    let mut entries = list_entries;
    entries.extend(others);
    for (index, entry) in entries.iter_mut().enumerate() {
        entry.priority = (index as u32 + 1) * 10;
    }
    report.packages = entries.len() as u32;
    load_order.entries = entries;
    load_order.content_order = Some(content);
    load_order.version = LOAD_ORDER_VERSION;
    if load_order.base_game_data.is_none() {
        load_order.base_game_data = crate::openmw_ini_importer::read_base_data_path();
    }
    save_load_order(data_dir, &load_order)?;

    // openmw.cfg lines: groundcover, the mods' archives and fallback values
    let mut lines: Vec<String> = cfg_values(&config.openmw_cfg.groundcover_files, "groundcover")
        .into_iter()
        .map(|file| format!("groundcover={file}"))
        .collect();
    lines.extend(
        cfg_values(&config.openmw_cfg.fallback_archives, "fallback-archive")
            .into_iter()
            .filter(|bsa| !BASE_ARCHIVES.iter().any(|base| base.eq_ignore_ascii_case(bsa)))
            .map(|bsa| format!("fallback-archive={bsa}")),
    );
    lines.extend(
        cfg_values(&config.openmw_cfg.fallback_values, "fallback").into_iter().map(|value| format!("fallback={value}")),
    );

    let mut settings = crate::instance_settings::load_instance_settings(data_dir)?;
    settings.openmw_cfg_overrides.retain(|line| !state.cfg_lines.contains(line));
    settings.openmw_cfg_overrides.extend(lines.iter().cloned());
    state.cfg_lines = lines;
    let own_settings = settings.openmw_settings_cfg_overrides.trim();
    if own_settings.is_empty() || own_settings == state.settings_cfg.trim() {
        settings.openmw_settings_cfg_overrides = config.settings_cfg.clone();
        state.settings_cfg = config.settings_cfg.clone();
    } else {
        report.notes.push(format!(
            "Kept your own settings.cfg overrides; {slug}'s recommended settings weren't applied"
        ));
    }
    crate::instance_settings::save_instance_settings(data_dir, &settings)
}

/// Downloads and installs the list's mods into the instance's data folder, then sets the instance's load order.
pub async fn install_modlist(
    data_dir: PathBuf,
    slug: String,
    beta: bool,
    link_handler: bool,
    progress: Option<ProgressEmitter>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
) -> Result<MomwInstallReport, String> {
    let progress = Arc::new(Mutex::new(progress));
    let emit = |phase: BackgroundOperationPhase, message: String, step: u64, total: u64, item: Option<String>| {
        if let Ok(mut guard) = progress.lock() {
            if let Some(emitter) = guard.as_mut() {
                emitter.emit(phase, message, step, total, item, true);
            }
        }
    };
    let cancelled = || cancel.load(std::sync::atomic::Ordering::Relaxed);

    emit(BackgroundOperationPhase::ResolvingMods, format!("Fetching {slug} from Modding-OpenMW.com"), 0, 1, None);
    let mods = fetch_modlist(&slug, beta).await?;
    let config = fetch_generated_config(&slug, beta).await?;
    crate::instance_data::ensure_instance_data_layout(&data_dir)?;

    let mut state = load_state(&data_dir);
    if state.modlist != slug {
        state.modlist = slug.clone();
        state.beta = beta;
    }
    let mut report = MomwInstallReport { modlist: slug.clone(), ..Default::default() };

    let nexus = NexusClient::from_stored_key();
    let premium = match &nexus {
        Some(client) => client.validate().await.map(|user| user.is_premium).unwrap_or(false),
        None => false,
    };
    let http = reqwest::Client::builder().user_agent(crate::nexus::user_agent()).build().map_err(|e| e.to_string())?;
    let mut resolver =
        Resolver { nexus: nexus.clone(), premium, link_handler, file_lists: HashMap::new(), http: http.clone() };

    // Find what each mod needs
    struct Pending {
        item: MomwMod,
        downloads: Vec<(DownloadInfo, Download)>,
    }
    let mut pending = Vec::new();
    let installable: Vec<&MomwMod> = mods.iter().filter(|m| !is_guide(m)).collect();
    report.guides = (mods.len() - installable.len()) as u32;
    let total = installable.len() as u64;
    for (index, item) in installable.iter().enumerate() {
        if cancelled() {
            return Err("Install cancelled".into());
        }
        emit(
            BackgroundOperationPhase::ResolvingMods,
            format!("Checking {} ({}/{total})", item.name, index + 1),
            index as u64 + 1,
            total,
            Some(item.name.clone()),
        );
        let mut downloads = Vec::new();
        let mut manual = None;
        for info in &item.download_info {
            match resolver.resolve(item, info).await {
                Ok(Source::Download(download)) => downloads.push((info.clone(), download)),
                Ok(Source::Manual(reason)) => {
                    manual = Some(reason);
                    break;
                }
                Err(error) => {
                    manual = Some(error);
                    break;
                }
            }
        }
        if let Some(reason) = manual {
            let folder = format!(
                "{}/{}",
                item.category,
                item.download_info.first().and_then(|i| i.extract_to.clone()).unwrap_or_else(|| item.dir.clone())
            );
            report.manual.push(MomwManualMod { name: item.name.clone(), url: item.url.clone(), reason, folder });
            continue;
        }
        let versions: Vec<String> = downloads.iter().map(|(_, d)| d.version.clone()).collect();
        let installed = state.mods.get(&item.slug) == Some(&versions)
            && downloads.iter().all(|(info, _)| {
                data_dir.join(&item.category).join(info.extract_to.clone().unwrap_or_else(|| item.dir.clone())).exists()
            });
        if installed {
            report.up_to_date += 1;
        } else {
            pending.push(Pending { item: (*item).clone(), downloads });
        }
    }

    // Download, then extract each mod as soon as its files are there
    let cache_dir = archive_cache_dir();
    let total = pending.len() as u64;
    let done = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let state = Arc::new(Mutex::new(state));
    let results: Vec<Result<(String, Vec<String>), MomwFailedMod>> = futures_util::stream::iter(pending.into_iter().map(|job| {
        let http = http.clone();
        let cache_dir = cache_dir.clone();
        let data_dir = data_dir.clone();
        let done = done.clone();
        let state = state.clone();
        let cancel = cancel.clone();
        let nexus = nexus.clone();
        let emit = &emit;
        async move {
            let fail = |error: String| MomwFailedMod { name: job.item.name.clone(), error };
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(fail("Install cancelled".into()));
            }
            let mut archives = Vec::new();
            for (info, download) in &job.downloads {
                let path = cache_dir.join(safe_file_name(&job.item.dir)).join(safe_file_name(&download.file_name));
                let mut download = download.clone();
                let cached = download.size.is_some_and(|size| std::fs::metadata(&path).is_ok_and(|m| m.len() == size));
                if let (Some(click), false) = (download.click.clone(), cached) {
                    let nexus = nexus.as_ref().ok_or_else(|| fail("Link your Nexus Mods account".into()))?;
                    emit(
                        BackgroundOperationPhase::InstallingMods,
                        format!("Click \"Mod Manager Download\" for {} on Nexus Mods", job.item.name),
                        done.load(std::sync::atomic::Ordering::Relaxed),
                        total,
                        Some(job.item.name.clone()),
                    );
                    download.url = link_from_click(nexus, &click, &cancel).await.map_err(fail)?;
                }
                let download = &download;
                download_file(&http, download, &path).await.map_err(fail)?;
                archives.push((info.clone(), download.clone(), path));
            }
            let item = job.item.clone();
            let notes = tauri::async_runtime::spawn_blocking({
                let data_dir = data_dir.clone();
                move || install_mod(&data_dir, &item, &archives)
            })
            .await
            .map_err(|e| fail(e.to_string()))?
            .map_err(fail)?;

            let versions = job.downloads.iter().map(|(_, d)| d.version.clone()).collect();
            if let Ok(mut state) = state.lock() {
                state.mods.insert(job.item.slug.clone(), versions);
                let _ = save_state(&data_dir, &state);
            }
            let finished = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            emit(
                BackgroundOperationPhase::InstallingMods,
                format!("Installed {} ({finished}/{total})", job.item.name),
                finished,
                total,
                Some(job.item.name.clone()),
            );
            Ok((job.item.name.clone(), notes))
        }
    }))
    // Free accounts click through the mods one at a time
    .buffer_unordered(if premium { PARALLEL_DOWNLOADS } else { 1 })
    .collect()
    .await;

    for result in results {
        match result {
            Ok((_, notes)) => {
                report.installed += 1;
                report.notes.extend(notes);
            }
            Err(failed) => report.failed.push(failed),
        }
    }

    emit(BackgroundOperationPhase::ApplyingLoadOrder, "Setting the load order from the list".into(), 0, 1, None);
    let mut state = Arc::try_unwrap(state).ok().and_then(|m| m.into_inner().ok()).unwrap_or_default();
    let data_dir_for_config = data_dir.clone();
    let (state, report) = tauri::async_runtime::spawn_blocking(move || {
        let mut report = report;
        apply_config(&data_dir_for_config, &slug, &config, &mut state, &mut report).map(|_| (state, report))
    })
    .await
    .map_err(|e| e.to_string())??;
    save_state(&data_dir, &state)?;
    emit(BackgroundOperationPhase::Complete, "Modlist installed".into(), 1, 1, None);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_sources() {
        assert_eq!(nexus_ids("https://www.nexusmods.com/morrowind/mods/54708"), Some(("morrowind".into(), 54708)));
        assert_eq!(nexus_ids("https://www.nexusmods.com/morrowind/mods/54708?tab=files"), Some(("morrowind".into(), 54708)));
        assert_eq!(gitlab_project("https://modding-openmw.gitlab.io/delta-plugin/"), Some("modding-openmw/delta-plugin".into()));
        assert_eq!(gitlab_project("https://gitlab.com/someone/some-mod/-/releases"), Some("someone/some-mod".into()));
    }

    #[test]
    fn applies_the_generated_config() {
        let data = std::env::temp_dir().join(format!("nerevar-momw-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(data.join("Cat/ModA/00 Core")).unwrap();
        std::fs::write(data.join("Cat/ModA/00 Core/a.esp"), "").unwrap();
        std::fs::write(data.join("Cat/ModA/00 Core/unused.esp"), "").unwrap();
        std::fs::create_dir_all(data.join("Tex/ModB")).unwrap();
        crate::instance_data::ensure_instance_data_layout(&data).unwrap();

        let config = super::super::GeneratedConfig {
            openmw_cfg: super::super::GeneratedOpenMwCfg {
                data_paths: "data=C:\\games\\OpenMWMods\\ModdingResources\\Morrowind\\Data Files\n\
                    data=C:\\games\\OpenMWMods\\Cat\\ModA\\00 Core\ndata=C:\\games\\OpenMWMods\\Tex\\ModB\n\
                    data=C:\\games\\OpenMWMods\\Missing\\Mod"
                    .into(),
                content_files: "content=Morrowind.esm\ncontent=a.esp".into(),
                groundcover_files: "groundcover=grass.esp".into(),
                fallback_archives: "fallback-archive=Morrowind.bsa\nfallback-archive=Mod.bsa".into(),
                fallback_values: String::new(),
            },
            settings_cfg: "[Shaders]\nforce shaders = true\n".into(),
        };
        let mut state = InstallState::default();
        let mut report = MomwInstallReport::default();
        apply_config(&data, "test", &config, &mut state, &mut report).unwrap();

        let order = load_load_order(&data).unwrap();
        let dirs: Vec<&str> = order.entries.iter().map(|e| e.relative_dir.as_str()).collect();
        assert_eq!(dirs, vec!["Cat/ModA/00 Core", "Tex/ModB"]);
        let plugins = &order.entries[0].plugins;
        assert!(plugins.iter().any(|p| p.file == "a.esp" && p.enabled));
        assert!(plugins.iter().any(|p| p.file == "unused.esp" && !p.enabled));
        assert_eq!(order.content_order.as_deref(), Some(&["Morrowind.esm".to_string(), "a.esp".to_string()][..]));
        assert_eq!(report.packages, 2);
        assert!(report.notes.iter().any(|n| n.contains("1 data folders")));

        let settings = crate::instance_settings::load_instance_settings(&data).unwrap();
        assert!(settings.openmw_cfg_overrides.contains(&"groundcover=grass.esp".to_string()));
        assert!(settings.openmw_cfg_overrides.contains(&"fallback-archive=Mod.bsa".to_string()));
        assert!(!settings.openmw_cfg_overrides.iter().any(|l| l.contains("Morrowind.bsa")));
        assert!(settings.openmw_settings_cfg_overrides.contains("force shaders"));

        // Applying again replaces this install's lines instead of adding them twice
        apply_config(&data, "test", &config, &mut state, &mut report).unwrap();
        let settings = crate::instance_settings::load_instance_settings(&data).unwrap();
        assert_eq!(settings.openmw_cfg_overrides.iter().filter(|l| *l == "groundcover=grass.esp").count(), 1);
        let _ = std::fs::remove_dir_all(&data);
    }
}
