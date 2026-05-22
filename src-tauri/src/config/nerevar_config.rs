use tauri::State;
use std::sync::Mutex;
use crate::AppState;
use std::path::Path;

pub fn get_nerevar_config_directory(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app_state = state.lock().unwrap();
    let config_path = app_state.nerevar_config_path.clone();
    let config_directory = Path::new(&config_path);

    if !config_directory.exists() {
        std::fs::create_dir_all(&config_directory).map_err(|e| e.to_string())?;
    }

    Ok(config_directory.to_string_lossy().to_string())
}

pub fn create_config_directory(config_directory: &str) -> Result<String, String> {
    let config_path = Path::new(config_directory);
    if !config_path.exists() {
        std::fs::create_dir_all(&config_path).map_err(|e| e.to_string())?;
    }
    Ok(config_path.to_string_lossy().to_string())
}