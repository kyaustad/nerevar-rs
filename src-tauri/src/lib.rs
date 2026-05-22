// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod data;
mod github_getters;
mod config;
use tauri::Manager;
use std::sync::Mutex;
use tauri::State;
use crate::data::GithubReleaseResponse;

#[derive(Default)]
struct AppState {
    nerevar_config_path: String,
}

#[tauri::command]
async fn get_all_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    github_getters::get_all_releases()
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_nerevar_config_directory(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    config::get_nerevar_config_directory(state)
}

#[tauri::command]
fn create_config_directory(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let app_state = state.lock().unwrap();
    match config::create_config_directory(&app_state.nerevar_config_path) {
        Ok(config_path) => Ok(config_path),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default().setup(|app| {
        app.manage(Mutex::new(AppState::default()));
        let app_state = app.state::<Mutex<AppState>>();
        let mut app_state = app_state.lock().unwrap();
        app_state.nerevar_config_path = app.path().app_data_dir().expect("Failed to get app data dir").join("config").to_string_lossy().to_string();
        Ok(())
    })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_all_releases, get_nerevar_config_directory, create_config_directory])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
