// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod config;
mod data;
mod file_actions;
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
    app_handle: Option<tauri::AppHandle>,
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

#[tauri::command]
async fn complete_onboarding(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    config::complete_onboarding(state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_directory_picker() -> Result<String, String> {
    file_actions::open_directory_picker().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_root_instance_path(
    state: State<'_, Mutex<AppState>>,
    path: String,
) -> Result<(), String> {
    config::set_root_instance_path(state, path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_sync_port(state: State<'_, Mutex<AppState>>, port: i32) -> Result<(), String> {
    config::set_sync_port(state, port)
        .await
        .map_err(|e| e.to_string())
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

            // Set the config path
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .nerevar_config_path = config::nerevar_config_file_path()
                .expect("Failed to resolve config path")
                .to_string_lossy()
                .to_string();

            let config = config::load_or_create_nerevar_config(app.state()).unwrap();

            // Set the config
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .nerevar_config = config;

            // Set the app Handle
            app.state::<Mutex<AppState>>().lock().unwrap().app_handle = Some(app.handle().clone());

            // DISABLED CONFIG WATCHER FOR NOW AS EVEN INTERNAL CHANGES TRIGGER IT AND WILL
            // CAUSE UNECESSARY RE-RENDERS IN REACT

            // config::spawn_config_file_watcher(app.handle().clone());

            let _nerevar_server_task = tauri::async_runtime::spawn(async move {
                let _ = nerevar_server::start_web_server().await;
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        // REGISTER COMMANDS HERE
        .invoke_handler(tauri::generate_handler![
            get_all_releases,
            load_or_create_nerevar_config,
            complete_onboarding,
            open_directory_picker,
            set_root_instance_path,
            set_sync_port,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
