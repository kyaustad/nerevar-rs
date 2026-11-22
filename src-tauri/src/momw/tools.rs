//! Plugins that Modding-OpenMW lists generate with the MOMW Tools Pack, like momw-configurator does: Delta Plugin
//! (delta-merged.omwaddon), GoCoverify (groundcover) and Lumenaire (Lumenaire.omwaddon). They run on the host
//! after an install, with a temporary openmw.cfg of the list's load order, and write into the tools pack's own data
//! folder, which players get with the other packages.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{cfg_values, is_base_game_data_path, relative_data_path, GeneratedConfig};
use crate::instance_data::package_abs_path;

const TOOLS_DIR: &str = "Tools/MOMWToolsPack";
const LUMENAIRE_PLUGIN: &str = "Lumenaire.omwaddon";
const LEGACY_LIGHTING_PLUGIN: &str = "S3LightFixes.omwaddon";
const DELTA_PLUGIN: &str = "delta-merged.omwaddon";
const DELETED_GROUNDCOVER: &str = "deleted_groundcover.omwaddon";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A tool of the pack: `delta_plugin.exe`, or a platform build such as `lumenaire-windows-amd64.exe`.
fn find_tool(tools_dir: &Path, name: &str) -> Option<PathBuf> {
    let entries: Vec<PathBuf> = std::fs::read_dir(tools_dir).ok()?.flatten().map(|e| e.path()).collect();
    let is_exe = |path: &Path| {
        path.is_file()
            && (cfg!(not(windows)) || path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe")))
    };
    entries
        .iter()
        .find(|path| is_exe(path) && path.file_stem().is_some_and(|stem| stem.to_string_lossy().eq_ignore_ascii_case(name)))
        .or_else(|| {
            entries.iter().find(|path| {
                is_exe(path)
                    && path.file_stem().is_some_and(|stem| {
                        stem.to_string_lossy().to_ascii_lowercase().starts_with(&format!("{}-", name.to_ascii_lowercase()))
                    })
            })
        })
        .cloned()
}

/// Content files in the root of a data folder: lowercased name -> path. Later folders win, like in OpenMW.
pub fn content_files_in(folder: &Path, found: &mut HashMap<String, PathBuf>) {
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_file() && crate::openmw_ini_importer::is_openmw_content_path(&path) {
            found.insert(entry.file_name().to_string_lossy().to_ascii_lowercase(), path);
        }
    }
}

/// The content files of a list (in load order) that can load: their file is there (`files`: lowercased name ->
/// path) and so are their masters, which load before them. A plugin with a missing master would stop the game
/// ("asks for parent file") and the tools. Returns those, lowercased, and each plugin left out with its master.
pub fn loadable_content(content: &[&str], files: &HashMap<String, PathBuf>) -> (HashSet<String>, Vec<(String, String)>) {
    let mut loadable = HashSet::new();
    let mut dropped = Vec::new();
    for name in content {
        let lower = name.to_ascii_lowercase();
        let Some(path) = files.get(&lower) else { continue };
        let masters = crate::openmw_ini_importer::esm_header::read_master_dependencies(path).unwrap_or_default();
        match masters.iter().find(|master| !loadable.contains(&master.to_ascii_lowercase())) {
            Some(master) => dropped.push((name.to_string(), master.clone())),
            None => {
                loadable.insert(lower);
            }
        }
    }
    (loadable, dropped)
}

/// The list's openmw.cfg with the instance's folders. `skip_content` lines are left out, like momw-configurator
/// comments them out while Delta Plugin merges, and so are content files that aren't installed (mods to download by
/// hand) or whose masters aren't: the tools refuse load orders with missing files. Returns the content files left out.
fn write_cfg(
    data_dir: &Path,
    base_data: Option<&str>,
    config: &GeneratedConfig,
    skip_content: &[&str],
    path: &Path,
) -> Result<Vec<String>, String> {
    let mut available = HashMap::new();
    let mut lines = vec!["# Temporary openmw.cfg of a Modding-OpenMW list, written by Nerevar".to_string()];
    for archive in cfg_values(&config.openmw_cfg.fallback_archives, "fallback-archive") {
        lines.push(format!("fallback-archive={archive}"));
    }
    for data in cfg_values(&config.openmw_cfg.data_paths, "data") {
        let folder = if is_base_game_data_path(data) {
            if let Some(base) = base_data {
                lines.push(format!("data=\"{base}\""));
                content_files_in(Path::new(base), &mut available);
            }
            continue;
        } else {
            match relative_data_path(data) {
                Some(relative) => package_abs_path(data_dir, &relative),
                None => continue,
            }
        };
        if folder.is_dir() {
            lines.push(format!("data=\"{}\"", folder.display()));
            content_files_in(&folder, &mut available);
        }
    }
    let content = cfg_values(&config.openmw_cfg.content_files, "content");
    let (loadable, _) = loadable_content(&content, &available);
    let mut missing = Vec::new();
    for content in content {
        if skip_content.iter().any(|skip| skip.eq_ignore_ascii_case(content)) {
            continue;
        }
        if loadable.contains(&content.to_ascii_lowercase()) {
            lines.push(format!("content={content}"));
        } else {
            missing.push(content.to_string());
        }
    }
    for groundcover in cfg_values(&config.openmw_cfg.groundcover_files, "groundcover") {
        if available.contains_key(&groundcover.to_ascii_lowercase()) {
            lines.push(format!("groundcover={groundcover}"));
        }
    }
    std::fs::write(path, lines.join("\n") + "\n").map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(missing)
}

fn run(tool: &Path, args: &[String], cwd: &Path) -> Result<(), String> {
    let mut command = Command::new(tool);
    command.args(args).current_dir(cwd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().map_err(|e| format!("Couldn't run {}: {e}", tool.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() { stdout } else { stderr };
    Err(format!(
        "{} failed ({}): {}",
        tool.file_name().unwrap_or_default().to_string_lossy(),
        output.status,
        detail.lines().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" / ")
    ))
}

/// tes3cmd's arguments for a clean action: `clean` by default; lists' own commands without the backup options,
/// like umo.
fn clean_arguments(arguments: Option<&serde_json::Value>) -> Vec<String> {
    let parsed: Option<Vec<String>> = match arguments {
        Some(serde_json::Value::Array(items)) => {
            Some(items.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
        }
        // Some lists keep the list as a string with single quotes
        Some(serde_json::Value::String(text)) => serde_json::from_str(&text.replace('\'', "\"")).ok(),
        _ => None,
    };
    match parsed {
        Some(args) if !args.is_empty() => {
            args.into_iter().filter(|a| !["--hide-backups", "tes3cmdbck", "--backup-dir"].contains(&a.as_str())).collect()
        }
        _ => vec!["clean".into()],
    }
}

/// A clean action of a mod: its plugin (relative to the category folder) and tes3cmd's arguments.
pub struct Clean {
    pub category_dir: PathBuf,
    pub path: String,
    pub arguments: Option<serde_json::Value>,
}

impl Clean {
    pub fn key(&self) -> String {
        format!("{}|{}|{}", self.category_dir.display(), self.path, self.arguments.as_ref().map(|a| a.to_string()).unwrap_or_default())
    }

    /// Plain cleaning writes `Clean_<plugin>` next to the plugin.
    pub fn output(&self) -> Option<PathBuf> {
        let args = clean_arguments(self.arguments.as_ref());
        if args != ["clean"] {
            return None;
        }
        let plugin = self.category_dir.join(&self.path);
        let name = plugin.file_name()?.to_string_lossy().into_owned();
        Some(plugin.with_file_name(format!("Clean_{name}")))
    }
}

/// Runs the clean actions with the MOMW Tools Pack's tes3cmd, from Morrowind's Data Files folder so tes3cmd finds
/// the masters.
pub fn clean(data_dir: &Path, base_data: Option<&str>, clean: &Clean) -> Result<(), String> {
    let tools_dir = package_abs_path(data_dir, TOOLS_DIR);
    let tes3cmd = find_tool(&tools_dir, "tes3cmd").ok_or("tes3cmd (MOMW Tools Pack) isn't installed")?;
    let plugin = clean.category_dir.join(&clean.path);
    if !plugin.is_file() {
        return Err(format!("{} isn't installed", plugin.display()));
    }
    let mut args = clean_arguments(clean.arguments.as_ref());
    args.push(plugin.to_string_lossy().into_owned());
    let cwd = base_data.map(PathBuf::from).filter(|p| p.is_dir()).unwrap_or(tools_dir);
    let mut command = Command::new(&tes3cmd);
    command.args(&args).current_dir(cwd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().map_err(|e| format!("Couldn't run tes3cmd: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || text.contains("FATAL ERROR") {
        let detail: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).rev().take(3).collect();
        return Err(format!("tes3cmd {} failed: {}", args[0], detail.into_iter().rev().collect::<Vec<_>>().join(" / ")));
    }
    if let Some(out) = clean.output() {
        if !out.is_file() {
            // tes3cmd writes nothing when there's nothing to clean: the plugin is clean already
            std::fs::copy(&plugin, &out).map_err(|e| format!("Failed to write {}: {e}", out.display()))?;
        }
    }
    Ok(())
}

/// Generates the list's plugins. Returns notes for the install report.
pub fn generate_plugins(data_dir: &Path, base_data: Option<&str>, config: &GeneratedConfig) -> Vec<String> {
    let content = config.openmw_cfg.content_files.to_ascii_lowercase();
    let use_delta = content.contains("delta-merged");
    let use_gocoverify = content.contains("deleted_groundcover");
    let use_lumenaire = content.contains("lumenaire");
    if !use_delta && !use_gocoverify && !use_lumenaire {
        return Vec::new();
    }

    let tools_dir = package_abs_path(data_dir, TOOLS_DIR);
    let mut notes = Vec::new();
    if !tools_dir.is_dir() {
        notes.push("The MOMW Tools Pack isn't installed, so the list's generated plugins are missing".into());
        return notes;
    }
    let cfg_path = crate::instance_data::nerevar_dir(data_dir).join("momw-openmw.cfg");
    let out_dir = tools_dir.to_string_lossy().into_owned();
    let cfg_arg = cfg_path.to_string_lossy().into_owned();

    let mut reported_missing = false;
    let mut step = |name: &str, tool: &str, skip: &[&str], args: Vec<String>| {
        let Some(exe) = find_tool(&tools_dir, tool) else {
            notes.push(format!("{name} isn't in the MOMW Tools Pack; its plugins are missing"));
            return;
        };
        let missing = match write_cfg(data_dir, base_data, config, skip, &cfg_path) {
            Ok(missing) => missing,
            Err(error) => {
                notes.push(format!("{name}: {error}"));
                return;
            }
        };
        // The list's generated plugins are missing until the tools make them
        let generated = [DELTA_PLUGIN, DELETED_GROUNDCOVER, LUMENAIRE_PLUGIN, LEGACY_LIGHTING_PLUGIN];
        let missing: Vec<String> =
            missing.into_iter().filter(|m| !generated.iter().any(|g| g.eq_ignore_ascii_case(m))).collect();
        if !missing.is_empty() && !reported_missing {
            reported_missing = true;
            notes.push(format!(
                "Generated the plugins without content files that aren't installed: {}",
                missing.join(", ")
            ));
        }
        match run(&exe, &args, &tools_dir) {
            Ok(()) => notes.push(format!("Generated the plugins of {name}")),
            Err(error) => notes.push(format!("{name}: {error}")),
        }
    };

    if use_delta {
        step(
            "Delta Plugin",
            "delta_plugin",
            &["LuaMultiMark.omwaddon", "S3maphore.esp", LEGACY_LIGHTING_PLUGIN, LUMENAIRE_PLUGIN, DELTA_PLUGIN, DELETED_GROUNDCOVER],
            vec![
                "-c".into(),
                cfg_arg.clone(),
                "merge".into(),
                "--skip".into(),
                "Cell".into(),
                "--skip".into(),
                "Dialogue".into(),
                "--skip".into(),
                "DialogueInfo".into(),
                format!("{out_dir}/{DELTA_PLUGIN}"),
            ],
        );
    }
    if use_gocoverify {
        step(
            "GoCoverify",
            "gocoverify",
            &[],
            vec![
                "--openmw-config".into(),
                cfg_arg.clone(),
                "--output-dir".into(),
                out_dir.clone(),
                "--ignore".into(),
                "LuaMultiMark.omwaddon".into(),
                "--ignore".into(),
                DELETED_GROUNDCOVER.into(),
                "--ignore".into(),
                LEGACY_LIGHTING_PLUGIN.into(),
                "--ignore".into(),
                LUMENAIRE_PLUGIN.into(),
            ],
        );
    }
    if use_lumenaire {
        step(
            "Lumenaire",
            "lumenaire",
            &[],
            vec!["--openmw-config".into(), cfg_arg.clone(), "--output-dir".into(), out_dir.clone()],
        );
    }
    let _ = std::fs::remove_file(&cfg_path);
    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_clean_arguments() {
        assert_eq!(clean_arguments(None), vec!["clean"]);
        let list = serde_json::json!(["delete", "--backup-dir", "tes3cmdbck", "--hide-backups", "--type", "cell"]);
        assert_eq!(clean_arguments(Some(&list)), vec!["delete", "--type", "cell"]);
        let text = serde_json::json!("['clean', '--cell-params']");
        assert_eq!(clean_arguments(Some(&text)), vec!["clean", "--cell-params"]);
        let action = Clean { category_dir: PathBuf::from("C:/data/Armor"), path: "Ebony/Ebony.ESP".into(), arguments: None };
        assert_eq!(action.output().unwrap(), PathBuf::from("C:/data/Armor/Ebony/Clean_Ebony.ESP"));
    }

    #[test]
    fn finds_platform_tools_and_writes_the_cfg() {
        let data = std::env::temp_dir().join(format!("nerevar-tools-{}", uuid::Uuid::new_v4()));
        let tools = data.join(TOOLS_DIR);
        std::fs::create_dir_all(&tools).unwrap();
        let exe = if cfg!(windows) { ".exe" } else { "" };
        std::fs::write(tools.join(format!("delta_plugin{exe}")), "").unwrap();
        std::fs::write(tools.join(format!("lumenaire-windows-amd64{exe}")), "").unwrap();
        assert!(find_tool(&tools, "delta_plugin").is_some());
        assert!(find_tool(&tools, "lumenaire").is_some());
        assert!(find_tool(&tools, "gocoverify").is_none());

        std::fs::create_dir_all(data.join("Cat/Mod")).unwrap();
        let config = GeneratedConfig {
            openmw_cfg: super::super::GeneratedOpenMwCfg {
                data_paths: "data=C:\\games\\OpenMWMods\\ModdingResources\\Morrowind\\Data Files\n\
                    data=C:\\games\\OpenMWMods\\Cat\\Mod\ndata=C:\\games\\OpenMWMods\\Tools\\MOMWToolsPack"
                    .into(),
                content_files: "content=Morrowind.esm\ncontent=a.esp\ncontent=Lumenaire.omwaddon".into(),
                fallback_archives: "fallback-archive=Morrowind.bsa".into(),
                ..Default::default()
            },
            settings_cfg: String::new(),
        };
        let cfg = data.join("test.cfg");
        std::fs::write(data.join("Cat/Mod/a.esp"), "").unwrap();
        let missing = write_cfg(&data, Some("D:/Morrowind/Data Files"), &config, &[LUMENAIRE_PLUGIN], &cfg).unwrap();
        let text = std::fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("data=\"D:/Morrowind/Data Files\""));
        assert!(text.contains("Cat") && text.contains("MOMWToolsPack"));
        assert!(text.contains("content=a.esp"));
        assert!(!text.contains("Lumenaire"));
        // Morrowind.esm isn't in the test's base folder
        assert_eq!(missing, vec!["Morrowind.esm".to_string()]);
        let _ = std::fs::remove_dir_all(&data);
    }
}

/// Runs the tools step on an installed list: `cargo test live_tools -- --ignored --nocapture` with
/// `NEREVAR_LIVE_LIST` and `NEREVAR_LIVE_DIR` (the instance's data folder).
#[cfg(test)]
mod live {
    #[tokio::test]
    #[ignore]
    async fn live_tools() {
        let slug = std::env::var("NEREVAR_LIVE_LIST").expect("set NEREVAR_LIVE_LIST");
        let dir = std::path::PathBuf::from(std::env::var("NEREVAR_LIVE_DIR").expect("set NEREVAR_LIVE_DIR"));
        let config = super::super::fetch_generated_config(&slug, false).await.unwrap();
        let base = crate::instance_data::load_load_order(&dir).ok().and_then(|o| o.base_game_data);
        let base = base.map(|b| b.trim_matches('"').to_string());
        for note in super::generate_plugins(&dir, base.as_deref(), &config) {
            println!("NOTE {note}");
        }
    }
}
