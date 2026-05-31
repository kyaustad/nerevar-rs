use std::path::Path;

use rfd::FileDialog;

const MO2_PLUGIN_FILENAME: &str = "export_modlist_with_directories.py";

const MO2_PLUGIN_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../scripts/mo2_plugin/export_modlist_with_directories.py"
));

#[tauri::command]
pub fn install_mo2_export_plugin(plugins_directory: Option<String>) -> Result<String, String> {
    let directory = match plugins_directory.filter(|path| !path.trim().is_empty()) {
        Some(path) => path,
        None => FileDialog::new()
            .set_title("Select your Mod Organizer 2 plugins folder")
            .pick_folder()
            .ok_or_else(|| "No directory selected".to_string())?
            .to_string_lossy()
            .into_owned(),
    };

    let plugins_dir = Path::new(&directory);
    if !plugins_dir.is_dir() {
        return Err(format!("Not a directory: {directory}"));
    }

    let destination = plugins_dir.join(MO2_PLUGIN_FILENAME);
    std::fs::write(&destination, MO2_PLUGIN_SOURCE).map_err(|error| {
        format!(
            "Failed to write {}: {error}",
            destination.to_string_lossy()
        )
    })?;

    Ok(destination.to_string_lossy().into_owned())
}
