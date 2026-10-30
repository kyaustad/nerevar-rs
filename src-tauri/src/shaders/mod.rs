//! OpenMW post-processing shaders (`shaders/*.omwfx`) available to an instance, and the
//! post-processing setup of the instance's own game, which hosts tune in-game (F2).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

use crate::instance_data::{find_instance_by_id, resolve_package_data_dir};
use crate::instance_settings::{parse_settings_cfg, POST_PROCESSING_SECTION, SHADER_CHAIN_KEY};
use crate::instance_setup::{instance_tes3mp_dir, tes3mp_config_dir};
use crate::AppState;

const SHADER_EXTENSION: &str = "omwfx";
/// OpenMW's own shaders that aren't meant for the chain.
const INTERNAL_PREFIX: &str = "internal_";
const BUILT_IN_SOURCE: &str = "TES3MP-NG";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShaderInfo {
    /// Name used in the chain: the file name without `.omwfx`.
    pub name: String,
    /// `TES3MP-NG` for built-in shaders, else the data folder's name.
    pub source: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceShaders {
    pub shaders: Vec<ShaderInfo>,
    /// The post-processing setup of the instance's own game (`config/settings.cfg`), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_chain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_post_processing_enabled: Option<bool>,
    /// The instance's own `config/shaders.yaml`, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_shaders_yaml: Option<String>,
}

/// Shaders in the release's `resources/vfs/shaders` and in each data folder's `shaders/`, later
/// folders overriding earlier ones like OpenMW's VFS.
pub fn find_shaders(tes3mp_dir: &Path, data_paths: &[PathBuf]) -> Vec<ShaderInfo> {
    let mut by_name: BTreeMap<String, ShaderInfo> = BTreeMap::new();
    let built_in = tes3mp_dir.join("resources").join("vfs").join("shaders");
    add_shaders(&mut by_name, &built_in, BUILT_IN_SOURCE);

    for dir in data_paths {
        let Some(shaders_dir) = find_child_dir(dir, "shaders") else {
            continue;
        };
        let source = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| dir.to_string_lossy().into_owned());
        add_shaders(&mut by_name, &shaders_dir, &source);
    }
    by_name.into_values().collect()
}

fn add_shaders(by_name: &mut BTreeMap<String, ShaderInfo>, dir: &Path, source: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        let is_shader = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case(SHADER_EXTENSION));
        let Some(name) = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()) else {
            continue;
        };
        if !is_shader || !path.is_file() || name.to_ascii_lowercase().starts_with(INTERNAL_PREFIX) {
            continue;
        }
        by_name.insert(
            name.to_ascii_lowercase(),
            ShaderInfo { name, source: source.to_string(), path: path.to_string_lossy().into_owned() },
        );
    }
}

/// A child folder by name, ignoring case (mods ship `Shaders/` or `shaders/`).
fn find_child_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|file| file.to_string_lossy().eq_ignore_ascii_case(name))
        })
}

/// The post-processing setup of a TES3MP-NG install's own game.
fn game_post_processing(config_dir: &Path) -> (Option<String>, Option<bool>, Option<String>) {
    let settings = std::fs::read_to_string(config_dir.join("settings.cfg"))
        .map(|text| parse_settings_cfg(&text))
        .unwrap_or_default();
    let section = settings.get(POST_PROCESSING_SECTION);
    let chain = section.and_then(|entries| entries.get(SHADER_CHAIN_KEY)).cloned();
    let enabled = section
        .and_then(|entries| entries.get("enabled"))
        .map(|value| value.eq_ignore_ascii_case("true"));
    let shaders_yaml = std::fs::read_to_string(config_dir.join("shaders.yaml")).ok();
    (chain, enabled, shaders_yaml)
}

#[tauri::command]
pub async fn scan_instance_shaders(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<InstanceShaders, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };
    let data_dir = resolve_package_data_dir(&instance);
    let tes3mp_dir = instance_tes3mp_dir(Path::new(&instance.path));

    tauri::async_runtime::spawn_blocking(move || {
        let resolved = crate::instance_data::resolve_instance_openmw_config(&data_dir)?;
        let data_paths: Vec<PathBuf> = resolved
            .data_paths
            .iter()
            .map(|path| crate::lua_mods::analyze::data_path_buf(path))
            .collect();
        let (game_chain, game_post_processing_enabled, game_shaders_yaml) =
            game_post_processing(&tes3mp_config_dir(&tes3mp_dir));
        Ok(InstanceShaders {
            shaders: find_shaders(&tes3mp_dir, &data_paths),
            game_chain,
            game_post_processing_enabled,
            game_shaders_yaml,
        })
    })
    .await
    .map_err(|error| format!("Shader scan failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_built_in_and_mod_shaders() {
        let root = std::env::temp_dir().join(format!("nerevar-shaders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let tes3mp = root.join("tes3mp");
        std::fs::create_dir_all(tes3mp.join("resources/vfs/shaders")).unwrap();
        std::fs::write(tes3mp.join("resources/vfs/shaders/bloomlinear.omwfx"), "").unwrap();
        std::fs::write(tes3mp.join("resources/vfs/shaders/internal_distortion.omwfx"), "").unwrap();
        let xe = root.join("data/01 XE-Shaders");
        std::fs::create_dir_all(xe.join("Shaders")).unwrap();
        std::fs::write(xe.join("Shaders/hdr.omwfx"), "").unwrap();
        std::fs::write(xe.join("Shaders/bloomlinear.omwfx"), "").unwrap();
        std::fs::write(xe.join("Shaders/readme.txt"), "").unwrap();

        let shaders = find_shaders(&tes3mp, &[xe.clone()]);
        let names: Vec<(&str, &str)> = shaders.iter().map(|s| (s.name.as_str(), s.source.as_str())).collect();
        assert_eq!(names, vec![("bloomlinear", "01 XE-Shaders"), ("hdr", "01 XE-Shaders")]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reads_the_game_post_processing_setup() {
        let dir = std::env::temp_dir().join(format!("nerevar-shaders-game-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.cfg"), "[Post Processing]\nenabled = true\nchain = hdr,bloomlinear\n").unwrap();
        std::fs::write(dir.join("shaders.yaml"), "hdr:\n  exposure: 1.2\n").unwrap();
        let (chain, enabled, yaml) = game_post_processing(&dir);
        assert_eq!(chain.as_deref(), Some("hdr,bloomlinear"));
        assert_eq!(enabled, Some(true));
        assert!(yaml.unwrap().contains("exposure"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
