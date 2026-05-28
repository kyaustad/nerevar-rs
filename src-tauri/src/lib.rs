// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod config;
mod data;
mod file_actions;
mod github_getters;
mod instance_setup;
mod nerevar_server;
mod openmw_ini_importer;

use crate::data::GithubReleaseResponse;
use crate::data::NerevarConfig;
use crate::data::NewInstanceConfig;
use std::sync::Mutex;
use tauri::Manager;
use tauri::State;
use tokio::sync::watch;

#[derive(Default)]
struct AppState {
    app_handle: Option<tauri::AppHandle>,
    nerevar_config_path: String,
    nerevar_config: NerevarConfig,
    server_port_tx: Option<watch::Sender<i32>>,
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
fn open_esm_file_picker() -> Result<String, String> {
    file_actions::open_esm_file_picker().map_err(|e| e.to_string())
}

#[tauri::command]
fn open_directory(path: String) -> Result<(), String> {
    file_actions::open_directory(path.to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_root_path(state: State<'_, Mutex<AppState>>, path: String) -> Result<(), String> {
    config::set_root_path(state, path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_sync_port(state: State<'_, Mutex<AppState>>, port: i32) -> Result<(), String> {
    config::set_sync_port(state, port)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_instance(
    state: State<'_, Mutex<AppState>>,
    new_instance: NewInstanceConfig,
) -> Result<(), String> {
    config::add_instance(state, new_instance).await
}

// #[tauri::command]
// async fn download_and_run_openmw_wizard(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
//     config::download_and_run_openmw_wizard(state)
//         .await
//         .map_err(|e| e.to_string())
// }

#[tauri::command]
async fn validate_global_openmw_config() -> Result<bool, String> {
    config::validate_global_openmw_config()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn generate_default_global_openmw_config(
    morrowind_installation_path: String,
) -> Result<(), String> {
    config::generate_default_global_openmw_config(morrowind_installation_path)
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

            // Start web server supervisor: it will restart when sync_port changes.
            let initial_port = app
                .state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .nerevar_config
                .sync_port;

            let (tx, mut rx) = watch::channel(initial_port);
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .server_port_tx = Some(tx);

            tauri::async_runtime::spawn(async move {
                let mut current_task: Option<tauri::async_runtime::JoinHandle<()>>;

                let start = |port: i32| {
                    tauri::async_runtime::spawn(async move {
                        if let Err(err) = nerevar_server::start_web_server_on_port(port).await {
                            tauri_plugin_log::log::error!(
                                "NEREVAR SERVER: failed to start on port {port}: {err}"
                            );
                        }
                    })
                };

                current_task = Some(start(*rx.borrow()));

                while rx.changed().await.is_ok() {
                    let next_port = *rx.borrow();
                    tauri_plugin_log::log::info!("NEREVAR SERVER: restarting on port {next_port}");

                    if let Some(task) = current_task.take() {
                        task.abort();
                    }

                    current_task = Some(start(next_port));
                }
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
            open_esm_file_picker,
            open_directory,
            set_root_path,
            set_sync_port,
            add_instance,
            validate_global_openmw_config,
            generate_default_global_openmw_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
