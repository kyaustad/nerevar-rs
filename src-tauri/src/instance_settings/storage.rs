use std::path::Path;

use super::types::InstanceSettings;

pub const INSTANCE_SETTINGS_FILE: &str = "instance-settings.json";

pub fn instance_settings_path(data_dir: &Path) -> std::path::PathBuf {
    crate::instance_data::nerevar_dir(data_dir).join(INSTANCE_SETTINGS_FILE)
}

pub fn launch_settings_overlay_path(data_dir: &Path) -> std::path::PathBuf {
    crate::instance_data::launch_settings_overlay_path(data_dir)
}

pub fn load_instance_settings(data_dir: &Path) -> Result<InstanceSettings, String> {
    crate::instance_data::ensure_instance_data_layout(data_dir)?;
    let path = instance_settings_path(data_dir);
    if !path.is_file() {
        return Ok(crate::instance_settings::defaults::default_instance_settings());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read instance settings: {e}"))?;
    let settings: InstanceSettings = serde_json::from_str(&contents)
        .map_err(|e| format!("Invalid instance settings JSON: {e}"))?;
    Ok(crate::instance_settings::defaults::normalize_instance_settings(settings))
}

pub fn save_instance_settings(data_dir: &Path, settings: &InstanceSettings) -> Result<(), String> {
    crate::instance_data::ensure_instance_data_layout(data_dir)?;
    let path = instance_settings_path(data_dir);
    let normalized =
        crate::instance_settings::defaults::normalize_instance_settings(settings.clone());
    let json = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write instance settings: {e}"))?;
    Ok(())
}

pub fn persist_settings_from_manifest(
    data_dir: &Path,
    settings: &InstanceSettings,
) -> Result<(), String> {
    save_instance_settings(data_dir, settings)?;
    write_launch_settings_overlay(data_dir, settings)
}

pub fn write_launch_settings_overlay(
    data_dir: &Path,
    settings: &InstanceSettings,
) -> Result<(), String> {
    crate::instance_data::ensure_instance_data_layout(data_dir)?;
    let path = launch_settings_overlay_path(data_dir);
    let contents = super::openmw_settings::format_settings_overlay(settings);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create launch settings directory: {e}"))?;
    }
    std::fs::write(&path, contents)
        .map_err(|e| format!("Failed to write launch settings overlay: {e}"))?;
    Ok(())
}
