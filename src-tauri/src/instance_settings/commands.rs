use std::path::Path;
use std::sync::Mutex;

use tauri::State;

use crate::instance_data::{find_instance_by_id, resolve_package_data_dir};
use crate::instance_setup::instance_tes3mp_dir;
use crate::AppState;

use super::defaults::{normalize_instance_settings, setting_definitions};
use super::storage::{
    load_instance_settings, save_instance_settings, write_launch_settings_overlay,
};
use super::tes3mp_lua::write_tes3mp_game_settings;
use super::types::{InstanceSettings, SettingDefinition};

pub fn apply_instance_settings_to_disk(
    instance_root: &Path,
    data_dir: &Path,
    settings: &InstanceSettings,
) -> Result<(), String> {
    save_instance_settings(data_dir, settings)?;
    write_launch_settings_overlay(data_dir, settings)?;
    write_tes3mp_game_settings(&instance_tes3mp_dir(instance_root), settings)?;
    Ok(())
}

#[tauri::command]
pub fn get_instance_setting_definitions() -> Vec<SettingDefinition> {
    setting_definitions()
}

#[tauri::command]
pub fn get_instance_settings(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<InstanceSettings, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };
    let data_dir = resolve_package_data_dir(&instance);
    load_instance_settings(&data_dir)
}

#[tauri::command]
pub fn save_instance_settings_command(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    settings: InstanceSettings,
) -> Result<InstanceSettings, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    if instance.remote_host.is_some() {
        return Err(
            "Synced instances inherit server settings from the host manifest. Sync from host to update."
                .to_string(),
        );
    }

    let data_dir = resolve_package_data_dir(&instance);
    let normalized = normalize_instance_settings(settings);
    apply_instance_settings_to_disk(Path::new(&instance.path), &data_dir, &normalized)?;
    Ok(normalized)
}
