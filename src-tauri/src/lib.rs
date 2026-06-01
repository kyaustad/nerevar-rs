// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_update;
mod config;
mod connection;
mod data;
mod file_actions;
mod github_getters;
mod instance_data;
mod instance_settings;
mod instance_setup;
mod mo2_plugin;
mod nerevar_server;
mod openmw_ini_importer;
mod port_conflict;
mod process_manager;
mod sync_auth;
mod sync_client;
mod sync_host;

use crate::data::GithubReleaseResponse;
use crate::data::NerevarConfig;
use crate::data::NewInstanceConfig;
use crate::port_conflict::PortConflict;
use crate::process_manager::ProcessManager;
use crate::sync_client::SyncCoordinator;
use crate::sync_host::new_shared_sync_host;
use std::sync::{Arc, Mutex};
use tauri::{Manager, RunEvent};
use tauri::State;
use tokio::sync::watch;

#[derive(Default)]
struct AppState {
    app_handle: Option<tauri::AppHandle>,
    nerevar_config_path: String,
    nerevar_config: NerevarConfig,
    server_port_tx: Option<watch::Sender<i32>>,
    server_retry_tx: Option<watch::Sender<u64>>,
    server_enabled_tx: Option<watch::Sender<bool>>,
    server_retry_generation: u64,
}

#[tauri::command]
async fn get_all_releases() -> Result<Vec<GithubReleaseResponse>, String> {
    github_getters::get_all_releases()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_app_version() -> String {
    app_update::current_app_version()
}

#[tauri::command]
async fn check_for_app_update() -> Result<app_update::AppUpdateStatus, String> {
    app_update::check_for_update().await
}

#[tauri::command]
async fn download_and_run_nerevar_update(
    app: tauri::AppHandle,
    release_id: u64,
) -> Result<(), String> {
    app_update::download_and_run_installer(release_id, app).await
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
fn open_csv_file_picker() -> Result<String, String> {
    file_actions::open_csv_file_picker().map_err(|e| e.to_string())
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

            let onboarding_complete = app
                .state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .nerevar_config
                .onboarding_complete;

            let (tx, mut rx) = watch::channel(initial_port);
            let (retry_tx, mut retry_rx) = watch::channel(0u64);
            let (enabled_tx, mut enabled_rx) = watch::channel(onboarding_complete);
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .server_port_tx = Some(tx);
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .server_retry_tx = Some(retry_tx);
            app.state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .server_enabled_tx = Some(enabled_tx);

            let sync_host = new_shared_sync_host();
            app.manage(sync_host.clone());
            app.manage(Arc::new(SyncCoordinator::new()));
            app.manage(Arc::new(ProcessManager::new()));

            let server_ctx = nerevar_server::state::ServerContext::new(sync_host);
            let app_handle = app.handle().clone();
            let startup_config = app
                .state::<Mutex<AppState>>()
                .lock()
                .unwrap()
                .nerevar_config
                .clone();

            if startup_config.onboarding_complete {
                tauri::async_runtime::spawn(async move {
                    if let Ok(conflicts) =
                        port_conflict::check_startup_conflicts(&startup_config)
                    {
                        port_conflict::emit_port_conflicts(&app_handle, conflicts);
                    }
                });
            }

            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let mut current_task: Option<tauri::async_runtime::JoinHandle<()>>;

                let start = |port: i32,
                             ctx: Arc<nerevar_server::state::ServerContext>,
                             app: tauri::AppHandle| {
                    tauri::async_runtime::spawn(async move {
                        match nerevar_server::try_bind(port).await {
                            Ok(listener) => {
                                if let Err(err) = nerevar_server::serve(listener, ctx).await {
                                    tauri_plugin_log::log::error!(
                                        "NEREVAR SERVER: stopped on port {port}: {err}"
                                    );
                                }
                            }
                            Err(err) => {
                                tauri_plugin_log::log::error!(
                                    "NEREVAR SERVER: failed to bind on port {port}: {err}"
                                );
                                if port_conflict::is_addr_in_use_error(&err) {
                                    match port_conflict::conflict_for_port(
                                        port as u16,
                                        port_conflict::PortRole::NerevarSync,
                                        None,
                                        None,
                                    ) {
                                        Ok(Some(conflict)) => {
                                            port_conflict::emit_port_conflicts(
                                                &app,
                                                vec![conflict],
                                            );
                                        }
                                        Ok(None) => {
                                            port_conflict::emit_port_conflicts(
                                                &app,
                                                vec![PortConflict {
                                                    port: port as u16,
                                                    role: port_conflict::PortRole::NerevarSync,
                                                    pid: 0,
                                                    process_name:
                                                        "Unknown process".to_string(),
                                                    executable_path: None,
                                                    instance_id: None,
                                                    instance_name: None,
                                                }],
                                            );
                                        }
                                        Err(parse_err) => {
                                            tauri_plugin_log::log::error!(
                                                "Failed to inspect port {port}: {parse_err}"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    })
                };

                let mut port = *rx.borrow();

                if *enabled_rx.borrow() {
                    tauri_plugin_log::log::info!(
                        "NEREVAR SERVER: starting on port {port}"
                    );
                    current_task =
                        Some(start(port, server_ctx.clone(), app_handle.clone()));
                } else {
                    tauri_plugin_log::log::info!(
                        "NEREVAR SERVER: waiting for onboarding to complete"
                    );
                    current_task = None;
                }

                loop {
                    tokio::select! {
                        changed = enabled_rx.changed() => {
                            if changed.is_err() {
                                break;
                            }
                            if !*enabled_rx.borrow() {
                                if let Some(task) = current_task.take() {
                                    task.abort();
                                }
                                current_task = None;
                                continue;
                            }

                            port = *rx.borrow();
                            tauri_plugin_log::log::info!(
                                "NEREVAR SERVER: onboarding complete — starting on port {port}"
                            );
                            if let Some(task) = current_task.take() {
                                task.abort();
                            }
                            current_task =
                                Some(start(port, server_ctx.clone(), app_handle.clone()));
                        }
                        changed = rx.changed() => {
                            if changed.is_err() {
                                break;
                            }
                            if !*enabled_rx.borrow() {
                                continue;
                            }
                            port = *rx.borrow();
                            tauri_plugin_log::log::info!(
                                "NEREVAR SERVER: restarting on port {port}"
                            );
                            if let Some(task) = current_task.take() {
                                task.abort();
                            }
                            current_task =
                                Some(start(port, server_ctx.clone(), app_handle.clone()));
                        }
                        changed = retry_rx.changed() => {
                            if changed.is_err() {
                                break;
                            }
                            if !*enabled_rx.borrow() {
                                continue;
                            }
                            tauri_plugin_log::log::info!(
                                "NEREVAR SERVER: retrying bind on port {port}"
                            );
                            if let Some(task) = current_task.take() {
                                task.abort();
                            }
                            current_task =
                                Some(start(port, server_ctx.clone(), app_handle.clone()));
                        }
                    }
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        // REGISTER COMMANDS HERE
        .invoke_handler(tauri::generate_handler![
            get_all_releases,
            get_app_version,
            check_for_app_update,
            download_and_run_nerevar_update,
            load_or_create_nerevar_config,
            complete_onboarding,
            open_directory_picker,
            open_csv_file_picker,
            open_esm_file_picker,
            open_directory,
            set_root_path,
            set_sync_port,
            add_instance,
            validate_global_openmw_config,
            generate_default_global_openmw_config,
            instance_data::commands::scan_instance_data,
            instance_data::commands::get_instance_load_order,
            instance_data::commands::save_instance_load_order,
            instance_data::commands::delete_instance_package,
            instance_data::commands::import_mo2_modlist_csv,
            instance_data::commands::resolve_instance_openmw,
            instance_data::commands::write_instance_launch_cfg,
            instance_data::commands::save_and_host_instance,
            instance_data::commands::build_instance_manifest,
            instance_data::commands::validate_instance_manifest,
            instance_data::commands::set_hosting_instance,
            sync_host::commands::activate_hosting_instance,
            sync_host::commands::clear_hosting_instance,
            sync_host::commands::get_sync_host_status,
            connection::commands::ping_remote_nerevar_server,
            connection::commands::fetch_remote_manifest_summary,
            connection::commands::add_synced_connection,
            connection::commands::sync_instance_from_remote,
            connection::commands::cancel_instance_sync,
            connection::commands::launch_instance_client,
            connection::commands::launch_instance_server,
            connection::commands::stop_instance_process,
            connection::commands::is_instance_process_running,
            connection::commands::get_global_process_status,
            connection::instance_edit::get_instance_connection_settings,
            connection::instance_edit::update_instance,
            connection::instance_delete::delete_instance,
            port_conflict::check_port_conflicts,
            port_conflict::kill_port_process,
            port_conflict::retry_sync_server,
            mo2_plugin::install_mo2_export_plugin,
            instance_settings::commands::get_instance_setting_definitions,
            instance_settings::commands::get_instance_settings,
            instance_settings::commands::save_instance_settings_command,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
                if let Some(manager) = app_handle.try_state::<Arc<ProcessManager>>() {
                    manager.restore_global_openmw_session_if_any();
                }
            }
        });
}
