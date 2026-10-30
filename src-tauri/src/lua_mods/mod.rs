//! OpenMW Lua mods of an instance: their `.omwscripts` manifests, the scripts they register and,
//! for global scripts, a suggested TES3MP-NG script mode (see `analyze`).

pub mod analyze;
pub mod commands;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use analyze::{ScriptModeSuggestion, SuggestionConfidence};

use analyze::{data_path_buf, resolve_vfs_path, suggest_script_mode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LuaScriptInfo {
    /// VFS path, such as `scripts/mod/world.lua`.
    pub path: String,
    /// Where OpenMW attaches it: `GLOBAL`, `PLAYER`, `NPC`, `MENU`, `CUSTOM`...
    pub tags: Vec<String>,
    pub is_global: bool,
    /// Whether the script file exists in the instance's data folders.
    pub found: bool,
    /// For global scripts: where the script should run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<ScriptModeSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LuaModInfo {
    /// The `.omwscripts` file name.
    pub file: String,
    /// The data folder that has it.
    pub data_path: String,
    /// Whether it's in the instance's content list.
    pub enabled: bool,
    pub scripts: Vec<LuaScriptInfo>,
}

/// Lua mods in `data_paths` (the instance's data folders, in load order), with `content` telling
/// which ones are enabled.
pub fn scan_lua_mods(data_paths: &[String], content: &[String]) -> Vec<LuaModInfo> {
    let dirs: Vec<PathBuf> = data_paths.iter().map(|path| data_path_buf(path)).collect();
    let mut mods = Vec::new();

    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut manifests: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("omwscripts"))
            })
            .collect();
        manifests.sort();

        for manifest in manifests {
            let file = manifest
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let Ok(text) = std::fs::read_to_string(&manifest) else {
                continue;
            };
            let scripts = parse_omwscripts(&text)
                .into_iter()
                .map(|(tags, path)| script_info(tags, path, &dirs))
                .collect();
            mods.push(LuaModInfo {
                enabled: content.iter().any(|entry| entry.eq_ignore_ascii_case(&file)),
                file,
                data_path: dir.to_string_lossy().into_owned(),
                scripts,
            });
        }
    }
    mods
}

/// OpenMW's own scripts, which TES3MP-NG always runs in client mode.
const BUILT_IN_SCRIPTS: &str = "scripts/omw/";

fn script_info(tags: Vec<String>, path: String, dirs: &[PathBuf]) -> LuaScriptInfo {
    let is_global = tags.iter().any(|tag| tag.eq_ignore_ascii_case("GLOBAL"));
    let found = resolve_vfs_path(&path, dirs).is_some();
    let built_in = path.to_ascii_lowercase().starts_with(BUILT_IN_SCRIPTS);
    let suggestion = if is_global && found && !built_in {
        suggest_script_mode(&path, dirs)
    } else {
        None
    };
    LuaScriptInfo { path, tags, is_global, found, suggestion }
}

/// `TAG, TAG: path` lines of an `.omwscripts` file. `#` starts a comment.
pub fn parse_omwscripts(text: &str) -> Vec<(Vec<String>, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.split('#').next().unwrap_or("").trim();
            let (tags, path) = line.split_once(':')?;
            let path = path.trim();
            if path.is_empty() {
                return None;
            }
            let tags = tags
                .split(',')
                .map(|tag| tag.trim().to_ascii_uppercase())
                .filter(|tag| !tag.is_empty())
                .collect();
            Some((tags, path.replace('\\', "/")))
        })
        .collect()
}

/// The data folders and content of an instance, for scanning its Lua mods.
pub fn instance_lua_mods(data_dir: &Path) -> Result<Vec<LuaModInfo>, String> {
    let resolved = crate::instance_data::resolve_instance_openmw_config(data_dir)?;
    let mut data_paths = Vec::new();
    // Morrowind's Data Files can have Lua mods too
    if let Some(base) = crate::openmw_ini_importer::read_base_data_path() {
        data_paths.push(base);
    }
    data_paths.extend(resolved.data_paths.iter().cloned());
    Ok(scan_lua_mods(&data_paths, &resolved.content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_settings::ScriptMode;

    /// Prints the Lua mods and suggestions for the data folders in `NEREVAR_TEST_LUA_DATA`
    /// (separated by `;`): `cargo test --lib -- --ignored --nocapture prints_lua_mod_suggestions`
    #[test]
    #[ignore]
    fn prints_lua_mod_suggestions() {
        let folders = std::env::var("NEREVAR_TEST_LUA_DATA").expect("set NEREVAR_TEST_LUA_DATA");
        let data_paths: Vec<String> = folders.split(';').map(str::to_string).collect();
        let content: Vec<String> = Vec::new();
        for lua_mod in scan_lua_mods(&data_paths, &content) {
            println!("== {} ({})", lua_mod.file, lua_mod.data_path);
            for script in lua_mod.scripts.iter().filter(|script| script.is_global) {
                match &script.suggestion {
                    Some(s) => println!("  {} -> {:?} ({:?}): {}", script.path, s.mode, s.confidence, s.reasons.join("; ")),
                    None => println!("  {} -> not found", script.path),
                }
            }
        }
    }

    #[test]
    fn parses_omwscripts_lines() {
        let text = "# A mod\nGLOBAL: scripts/mod/world.lua\nNPC, CREATURE: scripts/mod/actor.lua # actors\nPLAYER:scripts\\mod\\player.lua\n\nnot a script line\n";
        assert_eq!(
            parse_omwscripts(text),
            vec![
                (vec!["GLOBAL".to_string()], "scripts/mod/world.lua".to_string()),
                (vec!["NPC".to_string(), "CREATURE".to_string()], "scripts/mod/actor.lua".to_string()),
                (vec!["PLAYER".to_string()], "scripts/mod/player.lua".to_string()),
            ]
        );
    }

    #[test]
    fn scans_mods_and_suggests_modes_for_global_scripts() {
        let dir = std::env::temp_dir().join(format!("nerevar-lua-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let package = dir.join("Spawner");
        std::fs::create_dir_all(package.join("scripts/spawner")).unwrap();
        std::fs::write(
            package.join("Spawner.omwscripts"),
            "GLOBAL: scripts/spawner/world.lua\nPLAYER: scripts/spawner/hud.lua\nGLOBAL: scripts/spawner/missing.lua\n",
        )
        .unwrap();
        std::fs::write(
            package.join("scripts/spawner/world.lua"),
            "return { engineHandlers = { onUpdate = function() world.createObject('x') end } }",
        )
        .unwrap();
        std::fs::write(package.join("scripts/spawner/hud.lua"), "local ui = require('openmw.ui')").unwrap();

        let mods = scan_lua_mods(
            &[format!("\"{}\"", package.display())],
            &["Morrowind.esm".to_string(), "spawner.omwscripts".to_string()],
        );
        assert_eq!(mods.len(), 1);
        assert!(mods[0].enabled);
        let scripts = &mods[0].scripts;
        assert_eq!(scripts[0].suggestion.as_ref().unwrap().mode, ScriptMode::Authority);
        assert!(!scripts[1].is_global && scripts[1].found && scripts[1].suggestion.is_none());
        assert!(scripts[2].is_global && !scripts[2].found);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
