// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod config;
mod data;
mod github_getters;
mod nerevar_server;

use crate::data::GithubReleaseResponse;
use crate::data::NerevarConfig;
pub use nerevar_server::start_web_server;
use std::sync::Mutex;
use tauri::Manager;
use tauri::State;

#[derive(Default)]
struct AppState {
    nerevar_config_path: String,
    nerevar_config: NerevarConfig,
}

#[tauri::command]
async fn get_all_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    github_getters::get_all_releases()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn load_or_create_nerevar_config(
    state: State<'_, Mutex<AppState>>,
) -> Result<NerevarConfig, String> {
    config::load_or_create_nerevar_config(state)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            app.manage(Mutex::new(AppState::default()));
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .nerevar_config_path = config::nerevar_config_file_path()
                .expect("Failed to resolve config path")
                .to_string_lossy()
                .to_string();
            let config = config::load_or_create_nerevar_config(app.state()).unwrap();
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .nerevar_config = config;

            let _ = tauri::async_runtime::spawn(async move {
                let _ = nerevar_server::start_web_server().await;
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_all_releases,
            load_or_create_nerevar_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
