use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::State;

use crate::data::NerevarConfig;
use crate::AppState;

const CONFIG_FILE_NAME: &str = "config.json";

/// Same path as `app.path().app_data_dir()` / `config.json` (see Tauri `PathResolver::app_data_dir`).
pub fn nerevar_config_file_path() -> Result<PathBuf, String> {
    let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let identifier = context.config().identifier.clone();
    let app_data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to resolve app data directory".to_string())?
        .join(identifier);
    Ok(app_data_dir.join(CONFIG_FILE_NAME))
}

pub fn load_or_create_nerevar_config_at(config_path: &Path) -> Result<NerevarConfig, String> {
    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let default_config = NerevarConfig {
            onboarding_complete: false,
            instances: None,
            root_instance_path: None,
            base_tes3mp_path: None,
            sync_port: 25567,
        };
        std::fs::write(
            config_path,
            serde_json::to_string_pretty(&default_config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        return Ok(default_config);
    }

    let contents = std::fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

pub fn load_or_create_nerevar_config(
    state: State<'_, Mutex<AppState>>,
) -> Result<NerevarConfig, String> {
    let config_path = state.lock().unwrap().nerevar_config_path.clone();
    load_or_create_nerevar_config_at(Path::new(&config_path))
}
