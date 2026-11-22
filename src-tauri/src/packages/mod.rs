//! Packages the way Mod Organizer 2 handles mods: each mod is installed from its archive into its own folder of the
//! instance's data folder, from a file the player picks or drops, or from a Nexus Mods download; and the files that
//! packages overwrite in each other are listed (MO2's conflicts).

pub mod conflicts;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

use crate::instance_data::{find_instance_by_id, nerevar_dir, resolve_package_data_dir, scan_and_merge_load_order};
use crate::momw::archive;
use crate::nexus::nxm::NxmLink;
use crate::nexus::NexusClient;
use crate::AppState;

const SOURCES_FILE: &str = "package-sources.json";
const STAGING_DIR: &str = "staging";
/// Folders whose presence makes a folder a data folder
const DATA_DIR_NAMES: &[&str] = &[
    "meshes", "textures", "icons", "music", "sound", "bookart", "fonts", "video", "splash", "shaders", "scripts",
    "l10n", "mwse", "grass", "distantland",
];
const CONTENT_EXTENSIONS: &[&str] = &["esp", "esm", "omwaddon", "omwgame", "omwscripts", "bsa"];

/// Where an installed package came from, kept in `<data>/.nerevar/package-sources.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nexus_game: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nexus_mod_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nexus_file_id: Option<u64>,
    archive: String,
}

fn load_sources(data_dir: &Path) -> BTreeMap<String, PackageSource> {
    std::fs::read_to_string(nerevar_dir(data_dir).join(SOURCES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_sources(data_dir: &Path, sources: &BTreeMap<String, PackageSource>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(sources).map_err(|e| e.to_string())?;
    std::fs::write(nerevar_dir(data_dir).join(SOURCES_FILE), json).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PackageInstallResult {
    /// The package folder, relative to the data folder
    pub package: String,
    pub replaced: bool,
    pub notes: Vec<String>,
}

/// A folder name for a package.
pub fn package_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim().to_string();
    if cleaned.is_empty() || cleaned.starts_with('.') { "Mod".into() } else { cleaned }
}

fn is_data_root(dir: &Path) -> bool {
    std::fs::read_dir(dir).into_iter().flatten().flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let path = entry.path();
        if path.is_dir() {
            DATA_DIR_NAMES.contains(&name.as_str())
        } else {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| CONTENT_EXTENSIONS.iter().any(|c| ext.eq_ignore_ascii_case(c)))
        }
    })
}

/// The folder of an extracted archive that OpenMW should use: archives often wrap the data in a folder of the mod's
/// name or in "Data Files", like MO2 detects.
fn find_data_root(extracted: &Path) -> PathBuf {
    let mut root = extracted.to_path_buf();
    for _ in 0..4 {
        if is_data_root(&root) {
            break;
        }
        let entries: Vec<_> = std::fs::read_dir(&root).into_iter().flatten().flatten().collect();
        let dirs: Vec<_> = entries.iter().filter(|e| e.path().is_dir()).collect();
        if let Some(data_files) = dirs.iter().find(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            name == "data files" || name == "data"
        }) {
            root = data_files.path();
            continue;
        }
        let files = entries.iter().filter(|e| e.path().is_file()).count();
        if dirs.len() == 1 && files <= 3 {
            // A wrapping folder, maybe with a readme next to it
            root = dirs[0].path();
            continue;
        }
        break;
    }
    root
}

/// Installs an archive into `<data>/<name>`, replacing the package of that name.
pub fn install_archive(data_dir: &Path, archive_path: &Path, name: &str) -> Result<PackageInstallResult, String> {
    let folder = package_folder_name(name);
    let staging = nerevar_dir(data_dir).join(STAGING_DIR).join(uuid::Uuid::new_v4().to_string());
    let file_name = archive_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let result = (|| {
        archive::extract(archive_path, &staging, &file_name)?;
        let mut notes = Vec::new();
        if staging.join("fomod").is_dir() || find_data_root(&staging).join("fomod").is_dir() {
            notes.push(format!(
                "{name} has an installer with options (FOMOD); all of its files were installed, so check which folders \
                 OpenMW should use"
            ));
        }
        let root = find_data_root(&staging);
        let target = data_dir.join(&folder);
        let replaced = target.exists();
        if replaced {
            std::fs::remove_dir_all(&target).map_err(|e| format!("Failed to replace {}: {e}", target.display()))?;
        }
        std::fs::rename(&root, &target).map_err(|e| format!("Failed to install {}: {e}", target.display()))?;
        Ok(PackageInstallResult { package: folder.clone(), replaced, notes })
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn owned_data_dir(state: &State<'_, Mutex<AppState>>, instance_id: &str) -> Result<PathBuf, String> {
    let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
    let instance = find_instance_by_id(&guard.nerevar_config, instance_id)
        .ok_or_else(|| format!("Instance not found: {instance_id}"))?;
    if instance.remote_host.is_some() {
        return Err("Synced instances get their mods from their host".into());
    }
    Ok(resolve_package_data_dir(instance))
}

/// Installs a mod archive the player picked or dropped as a package, named after the archive unless `name` is given.
#[tauri::command]
pub async fn install_archive_package(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    archive_path: String,
    name: Option<String>,
) -> Result<PackageInstallResult, String> {
    let data_dir = owned_data_dir(&state, &instance_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::instance_data::ensure_instance_data_layout(&data_dir)?;
        let archive_path = PathBuf::from(archive_path);
        let name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| {
            archive_path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Mod".into())
        });
        let result = install_archive(&data_dir, &archive_path, &name)?;
        let mut sources = load_sources(&data_dir);
        sources.insert(result.package.clone(), PackageSource {
            nexus_game: None,
            nexus_mod_id: None,
            nexus_file_id: None,
            archive: archive_path.to_string_lossy().into_owned(),
        });
        save_sources(&data_dir, &sources)?;
        scan_and_merge_load_order(&data_dir, &mut None)?;
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Downloads the file of an nxm link and installs it into the instance: as the package of that Nexus mod when the
/// instance has one (an update), else as a new package named after the mod.
pub async fn install_nexus_link(app: &AppHandle, instance_id: &str, link: &NxmLink) -> Result<String, String> {
    let data_dir = owned_data_dir(&app.state::<Mutex<AppState>>(), instance_id)?;
    let nexus = NexusClient::from_stored_key()
        .ok_or_else(|| "Link your Nexus Mods account in Nerevar's settings to download from Nexus".to_string())?;
    let url = nexus
        .download_link_with_key(&link.game, link.mod_id, link.file_id, link.key.as_deref(), link.expires.as_deref())
        .await?;
    let files = nexus.files(&link.game, link.mod_id).await?;
    let file = files.files.iter().find(|f| f.file_id == link.file_id);
    let file_name = file.map(|f| f.file_name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("{}.archive", link.file_id));

    let sources = load_sources(&data_dir);
    let existing = sources
        .iter()
        .find(|(_, source)| source.nexus_mod_id == Some(link.mod_id) && source.nexus_game.as_deref() == Some(&link.game))
        .map(|(folder, _)| folder.clone());
    let name = match existing.clone() {
        Some(folder) => folder,
        None => nexus
            .mod_details(&link.game, link.mod_id)
            .await
            .ok()
            .and_then(|details| details.name)
            .unwrap_or_else(|| file.map(|f| f.name.clone()).unwrap_or_else(|| format!("Nexus mod {}", link.mod_id))),
    };

    // Into the downloads folder that modlist installs use, unless it's there already
    let downloads_dir = {
        let state = app.state::<Mutex<AppState>>();
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        crate::config::nerevar_config::downloads_dir(&guard.nerevar_config)
    };
    let downloads = crate::momw::downloads::Downloads::open(downloads_dir)?;
    let key = format!("nexus:{}", link.file_id);
    let size = file.and_then(|f| f.size_in_bytes);
    let archive_path = match downloads.cached(&key, size) {
        Some(path) => path,
        None => {
            let download =
                crate::momw::install::Download { url, file_name: file_name.clone(), version: key.clone(), size, nexus: None };
            let path = downloads.path_for(&key, &file_name);
            let http =
                reqwest::Client::builder().user_agent(crate::nexus::user_agent()).build().map_err(|e| e.to_string())?;
            crate::momw::install::download_file(&http, &download, &path).await?;
            downloads.record(&key, &path)?;
            path
        }
    };

    let link = link.clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::instance_data::ensure_instance_data_layout(&data_dir)?;
        let result = install_archive(&data_dir, &archive_path, &name)?;
        let mut sources = load_sources(&data_dir);
        sources.insert(result.package.clone(), PackageSource {
            nexus_game: Some(link.game.clone()),
            nexus_mod_id: Some(link.mod_id),
            nexus_file_id: Some(link.file_id),
            archive: archive_path.to_string_lossy().into_owned(),
        });
        save_sources(&data_dir, &sources)?;
        scan_and_merge_load_order(&data_dir, &mut None)?;
        Ok(result.package)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip(path: &Path, files: &[&str]) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for file in files {
            writer.start_file(*file, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn installs_archives_like_mo2() {
        let root = std::env::temp_dir().join(format!("nerevar-packages-{}", uuid::Uuid::new_v4()));
        let data = root.join("data");
        crate::instance_data::ensure_instance_data_layout(&data).unwrap();

        // Data wrapped in a folder of the mod's name, with a readme next to it
        zip(&root.join("wrapped.zip"), &["Better Rocks/textures/rock.dds", "Better Rocks/rocks.esp", "readme.txt"]);
        let result = install_archive(&data, &root.join("wrapped.zip"), "Better Rocks").unwrap();
        assert_eq!(result.package, "Better Rocks");
        assert!(data.join("Better Rocks/rocks.esp").is_file());
        assert!(data.join("Better Rocks/textures/rock.dds").is_file());

        // Data in "Data Files"
        zip(&root.join("datafiles.zip"), &["Mod/Data Files/meshes/a.nif", "Mod/Docs/readme.txt"]);
        install_archive(&data, &root.join("datafiles.zip"), "Mod: Name?").unwrap();
        assert!(data.join("Mod_ Name_/meshes/a.nif").is_file());

        // Installing again replaces the package
        let again = install_archive(&data, &root.join("wrapped.zip"), "Better Rocks").unwrap();
        assert!(again.replaced);
        assert!(!nerevar_dir(&data).join(STAGING_DIR).read_dir().unwrap().any(|_| true));
        let _ = std::fs::remove_dir_all(&root);
    }
}
