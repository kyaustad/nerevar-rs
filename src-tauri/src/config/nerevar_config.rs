use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::data::NerevarConfig;
use crate::AppState;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_log::log::info;

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
        info!("Creating default config file at {}", config_path.display());
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let default_config = NerevarConfig {
            onboarding_complete: false,
            instances: None,
            root_instance_path: None,
            sync_port: 25567,
        };
        std::fs::write(
            config_path,
            serde_json::to_string_pretty(&default_config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        info!("Default config file created at {}", config_path.display());
        return Ok(default_config);
    }

    let contents = std::fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    info!("Loading config file from {}", config_path.display());
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

pub fn load_or_create_nerevar_config(
    state: State<'_, Mutex<AppState>>,
) -> Result<NerevarConfig, String> {
    let config_path = state.lock().unwrap().nerevar_config_path.clone();
    load_or_create_nerevar_config_at(Path::new(&config_path))
}

pub async fn complete_onboarding(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let (app_handle, config) = {
        let mut state = state.lock().unwrap();
        state.nerevar_config.onboarding_complete = true;
        std::fs::write(
            Path::new(&state.nerevar_config_path),
            serde_json::to_string_pretty(&state.nerevar_config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        tauri_plugin_log::log::info!("Onboarding marked as complete in config and app state");

        (
            state
                .app_handle
                .clone()
                .ok_or_else(|| "App handle not initialized".to_string())?,
            state.nerevar_config.clone(),
        )
    };

    app_handle
        .emit("on_config_change", config)
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn spawn_config_file_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let config_path = app
            .state::<Mutex<AppState>>()
            .lock()
            .expect("config state poisoned")
            .nerevar_config_path
            .clone();

        let _ = tauri::async_runtime::spawn_blocking(move || {
            let path = PathBuf::from(&config_path);
            let watch_path = path.clone();

            let mut watcher = RecommendedWatcher::new(
                move |result: Result<notify::Event, notify::Error>| {
                    let Ok(event) = result else { return };
                    if !matches!(event.kind, EventKind::Modify(_)) {
                        return;
                    }

                    let Ok(config) = load_or_create_nerevar_config_at(&watch_path) else {
                        return;
                    };

                    let state = app.state::<Mutex<AppState>>();
                    if let Ok(mut app_state) = state.lock() {
                        app_state.nerevar_config = config.clone();
                    }

                    let _ = app.emit("on_config_change", config);
                    info!("Config file changed externally, emitting event and updating app state");
                },
                notify::Config::default(),
            )
            .expect("failed to create config watcher");

            watcher
                .watch(&path, RecursiveMode::NonRecursive)
                .expect("failed to watch config file");

            info!("Watching config file at {}", path.display());

            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        })
        .await;
    });
}

pub async fn set_root_instance_path(
    state: State<'_, Mutex<AppState>>,
    path: String,
) -> Result<(), String> {
    let mut state = state.lock().unwrap();
    state.nerevar_config.root_instance_path = Some(path);
    std::fs::write(
        Path::new(&state.nerevar_config_path),
        serde_json::to_string_pretty(&state.nerevar_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn set_sync_port(state: State<'_, Mutex<AppState>>, port: i32) -> Result<(), String> {
    let mut state = state.lock().unwrap();
    state.nerevar_config.sync_port = port;
    std::fs::write(
        Path::new(&state.nerevar_config_path),
        serde_json::to_string_pretty(&state.nerevar_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
