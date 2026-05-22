// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod data;
mod github_getters;
use crate::data::GithubReleaseResponse;

#[tauri::command]
async fn get_all_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    github_getters::get_all_releases()
    .await
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_all_releases])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
