//! Plugins that Modding-OpenMW lists generate with the MOMW Tools Pack, like momw-configurator does: Delta Plugin
//! (delta-merged.omwaddon), GoCoverify (groundcover) and Lumenaire (Lumenaire.omwaddon). They run on the host
//! after an install, with a temporary openmw.cfg of the list's load order, and write into the tools pack's own data
//! folder, which players get with the other packages.

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

/// The list's openmw.cfg with the instance's folders. `skip_content` lines are left out, like momw-configurator
/// comments them out while Delta Plugin merges.
fn write_cfg(
    data_dir: &Path,
    base_data: Option<&str>,
    config: &GeneratedConfig,
    skip_content: &[&str],
    path: &Path,
) -> Result<(), String> {
    let mut lines = vec!["# Temporary openmw.cfg of a Modding-OpenMW list, written by Nerevar".to_string()];
    for archive in cfg_values(&config.openmw_cfg.fallback_archives, "fallback-archive") {
        lines.push(format!("fallback-archive={archive}"));
    }
    for data in cfg_values(&config.openmw_cfg.data_paths, "data") {
        let folder = if is_base_game_data_path(data) {
            if let Some(base) = base_data {
                lines.push(format!("data=\"{base}\""));
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
        }
    }
    for content in cfg_values(&config.openmw_cfg.content_files, "content") {
        if !skip_content.iter().any(|skip| skip.eq_ignore_ascii_case(content)) {
            lines.push(format!("content={content}"));
        }
    }
    for groundcover in cfg_values(&config.openmw_cfg.groundcover_files, "groundcover") {
        lines.push(format!("groundcover={groundcover}"));
    }
    std::fs::write(path, lines.join("\n") + "\n").map_err(|e| format!("Failed to write {}: {e}", path.display()))
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

    let mut step = |name: &str, tool: &str, skip: &[&str], args: Vec<String>| {
        let Some(exe) = find_tool(&tools_dir, tool) else {
            notes.push(format!("{name} isn't in the MOMW Tools Pack; its plugins are missing"));
            return;
        };
        if let Err(error) = write_cfg(data_dir, base_data, config, skip, &cfg_path).and_then(|_| run(&exe, &args, &tools_dir)) {
            notes.push(format!("{name}: {error}"));
        } else {
            notes.push(format!("Generated the plugins of {name}"));
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
        write_cfg(&data, Some("D:/Morrowind/Data Files"), &config, &[LUMENAIRE_PLUGIN], &cfg).unwrap();
        let text = std::fs::read_to_string(&cfg).unwrap();
        assert!(text.contains("data=\"D:/Morrowind/Data Files\""));
        assert!(text.contains("Cat") && text.contains("MOMWToolsPack"));
        assert!(text.contains("content=a.esp"));
        assert!(!text.contains("Lumenaire"));
        let _ = std::fs::remove_dir_all(&data);
    }
}
