use std::path::{Path, PathBuf};
use std::sync::Mutex;
// use std::time::Duration;

use crate::data::{InstanceConfig, NerevarConfig, NewConnectionConfig, NewInstanceConfig};
use crate::port_conflict;
use crate::github_getters;
use crate::instance_data::ensure_instance_data_layout;
use crate::instance_setup::{apply_server_defaults, create_instance_data_dir, instance_tes3mp_dir};
use crate::AppState;
// use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_log::log::info;
use uuid::Uuid;

const CONFIG_FILE_NAME: &str = "config.json";
// const TES3MP_081_RELEASE_ID: &str = "65767406";

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
            owned_instances: None,
            synced_instances: None,
            root_path: None,
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
    let (app_handle, config, start_sync_server) = {
        let mut state = state.lock().unwrap();
        let was_complete = state.nerevar_config.onboarding_complete;
        state.nerevar_config.onboarding_complete = true;
        std::fs::write(
            Path::new(&state.nerevar_config_path),
            serde_json::to_string_pretty(&state.nerevar_config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        tauri_plugin_log::log::info!("Onboarding marked as complete in config and app state");

        if !was_complete {
            start_sync_server_supervisor(&mut state);
        }

        (
            state
                .app_handle
                .clone()
                .ok_or_else(|| "App handle not initialized".to_string())?,
            state.nerevar_config.clone(),
            !was_complete,
        )
    };

    app_handle
        .emit("on_config_change", config.clone())
        .map_err(|e| e.to_string())?;

    if start_sync_server {
        let app = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Ok(conflicts) = port_conflict::check_startup_conflicts(&config) {
                port_conflict::emit_port_conflicts(&app, conflicts);
            }
        });
    }

    Ok(())
}

fn start_sync_server_supervisor(state: &mut AppState) {
    if let Some(tx) = state.server_enabled_tx.clone() {
        let _ = tx.send(true);
    }

    let port = state.nerevar_config.sync_port;
    if let Some(tx) = state.server_port_tx.clone() {
        let _ = tx.send(port);
    }

    let next_retry = state.server_retry_generation.wrapping_add(1);
    state.server_retry_generation = next_retry;
    if let Some(tx) = state.server_retry_tx.clone() {
        let _ = tx.send(next_retry);
    }
}

// pub fn spawn_config_file_watcher(app: AppHandle) {
//     tauri::async_runtime::spawn(async move {
//         let config_path = app
//             .state::<Mutex<AppState>>()
//             .lock()
//             .expect("config state poisoned")
//             .nerevar_config_path
//             .clone();

//         let _ = tauri::async_runtime::spawn_blocking(move || {
//             let path = PathBuf::from(&config_path);
//             let watch_path = path.clone();

//             let mut watcher = RecommendedWatcher::new(
//                 move |result: Result<notify::Event, notify::Error>| {
//                     let Ok(event) = result else { return };
//                     if !matches!(event.kind, EventKind::Modify(_)) {
//                         return;
//                     }

//                     let Ok(config) = load_or_create_nerevar_config_at(&watch_path) else {
//                         return;
//                     };

//                     let state = app.state::<Mutex<AppState>>();
//                     if let Ok(mut app_state) = state.lock() {
//                         app_state.nerevar_config = config.clone();
//                     }

//                     let _ = app.emit("on_config_change", config);
//                     info!("Config file changed externally, emitting event and updating app state");
//                 },
//                 notify::Config::default(),
//             )
//             .expect("failed to create config watcher");

//             watcher
//                 .watch(&path, RecursiveMode::NonRecursive)
//                 .expect("failed to watch config file");

//             info!("Watching config file at {}", path.display());

//             loop {
//                 std::thread::sleep(Duration::from_secs(3600));
//             }
//         })
//         .await;
//     });
// }

pub async fn set_root_path(state: State<'_, Mutex<AppState>>, path: String) -> Result<(), String> {
    let mut state = state.lock().unwrap();
    state.nerevar_config.root_path = Some(path);
    std::fs::write(
        Path::new(&state.nerevar_config_path),
        serde_json::to_string_pretty(&state.nerevar_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn set_sync_port(state: State<'_, Mutex<AppState>>, port: i32) -> Result<(), String> {
    if !(1..=65535).contains(&port) {
        return Err(format!("Sync port must be between 1 and 65535, got {port}"));
    }

    let (config_path, config_snapshot, tx, app_handle, restart_server) = {
        let mut guard = state
            .lock()
            .map_err(|_| "App state lock poisoned".to_string())?;
        guard.nerevar_config.sync_port = port;
        (
            guard.nerevar_config_path.clone(),
            guard.nerevar_config.clone(),
            guard.server_port_tx.clone(),
            guard.app_handle.clone(),
            guard.nerevar_config.onboarding_complete,
        )
    };

    std::fs::write(
        Path::new(&config_path),
        serde_json::to_string_pretty(&config_snapshot).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    if restart_server {
        if let Some(tx) = tx {
            let _ = tx.send(port);
        }
    }

    if let Some(app) = app_handle {
        let _ = app.emit("on_config_change", config_snapshot);
    }

    Ok(())
}

fn new_instance_id() -> String {
    Uuid::new_v4().to_string()
}

fn build_instance_config(new_instance: &NewInstanceConfig) -> InstanceConfig {
    InstanceConfig {
        id: new_instance_id(),
        name: new_instance.instance_name.clone(),
        description: new_instance.instance_description.clone(),
        path: new_instance.instance_root_path.clone(),
        data_dir: new_instance.instance_data_dir.clone(),
        release_id: Some(new_instance.release_id.clone()),
        remote_host: None,
        remote_sync_port: None,
        last_synced_at: None,
        tes3mp_server_port: None,
        sync_password: None,
    }
}

pub fn build_synced_instance_config(new_connection: &NewConnectionConfig) -> InstanceConfig {
    InstanceConfig {
        id: new_instance_id(),
        name: new_connection.connection_name.clone(),
        description: new_connection.connection_description.clone(),
        path: new_connection.instance_root_path.clone(),
        data_dir: new_connection.instance_data_dir.clone(),
        release_id: Some(new_connection.release_id.clone()),
        remote_host: Some(new_connection.remote_host.clone()),
        remote_sync_port: Some(new_connection.remote_sync_port),
        last_synced_at: None,
        tes3mp_server_port: None,
        sync_password: Some(new_connection.sync_password.clone()),
    }
}

fn persist_owned_instance_to_config(
    state: &State<'_, Mutex<AppState>>,
    instance: InstanceConfig,
) -> Result<(NerevarConfig, AppHandle), String> {
    let mut guard = state
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;

    match guard.nerevar_config.owned_instances {
        Some(ref mut owned_instances) => owned_instances.push(instance),
        None => guard.nerevar_config.owned_instances = Some(vec![instance]),
    }

    std::fs::write(
        Path::new(&guard.nerevar_config_path),
        serde_json::to_string_pretty(&guard.nerevar_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let config = guard.nerevar_config.clone();
    let app_handle = guard
        .app_handle
        .clone()
        .ok_or_else(|| "App handle not initialized".to_string())?;

    Ok((config, app_handle))
}

pub fn persist_synced_instance_to_config(
    state: &State<'_, Mutex<AppState>>,
    instance: InstanceConfig,
) -> Result<(NerevarConfig, AppHandle), String> {
    let mut guard = state
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;

    match guard.nerevar_config.synced_instances {
        Some(ref mut synced_instances) => synced_instances.push(instance),
        None => guard.nerevar_config.synced_instances = Some(vec![instance]),
    }

    std::fs::write(
        Path::new(&guard.nerevar_config_path),
        serde_json::to_string_pretty(&guard.nerevar_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let config = guard.nerevar_config.clone();
    let app_handle = guard
        .app_handle
        .clone()
        .ok_or_else(|| "App handle not initialized".to_string())?;

    Ok((config, app_handle))
}

pub fn update_synced_instance(
    state: &State<'_, Mutex<AppState>>,
    instance: InstanceConfig,
) -> Result<NerevarConfig, String> {
    let mut guard = state
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;

    let synced = guard
        .nerevar_config
        .synced_instances
        .as_mut()
        .ok_or_else(|| "No synced instances configured".to_string())?;

    let entry = synced
        .iter_mut()
        .find(|i| i.id == instance.id)
        .ok_or_else(|| format!("Synced instance not found: {}", instance.id))?;
    *entry = instance;

    std::fs::write(
        Path::new(&guard.nerevar_config_path),
        serde_json::to_string_pretty(&guard.nerevar_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(guard.nerevar_config.clone())
}

pub fn update_owned_instance(
    state: &State<'_, Mutex<AppState>>,
    instance: InstanceConfig,
) -> Result<NerevarConfig, String> {
    let mut guard = state
        .lock()
        .map_err(|_| "App state lock poisoned".to_string())?;

    let owned = guard
        .nerevar_config
        .owned_instances
        .as_mut()
        .ok_or_else(|| "No owned instances configured".to_string())?;

    let entry = owned
        .iter_mut()
        .find(|i| i.id == instance.id)
        .ok_or_else(|| format!("Owned instance not found: {}", instance.id))?;
    *entry = instance;

    std::fs::write(
        Path::new(&guard.nerevar_config_path),
        serde_json::to_string_pretty(&guard.nerevar_config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(guard.nerevar_config.clone())
}

fn cleanup_failed_instance_root(path: &Path) {
    if path.exists() {
        if let Err(err) = std::fs::remove_dir_all(path) {
            info!(
                "Failed to clean up instance directory at {}: {err}",
                path.display()
            );
        }
    }
}

pub async fn add_instance(
    state: State<'_, Mutex<AppState>>,
    new_instance: NewInstanceConfig,
) -> Result<(), String> {
    let instance_root = Path::new(&new_instance.instance_root_path);
    if instance_root.exists() {
        return Err(format!(
            "Instance path already exists: {}",
            instance_root.display()
        ));
    }

    // Filesystem setup first; only persist config after success.
    if let Err(err) = (async {
        std::fs::create_dir_all(instance_root).map_err(|e| e.to_string())?;
        let instance_data_dir = Path::new(&new_instance.instance_data_dir);
        create_instance_data_dir(instance_data_dir)?;
        ensure_instance_data_layout(instance_data_dir)?;

        let tes3mp_dir = instance_tes3mp_dir(instance_root);
        std::fs::create_dir_all(&tes3mp_dir).map_err(|e| e.to_string())?;

        github_getters::download_and_extract_release_zip_by_id_to_path(
            new_instance.release_id.clone(),
            tes3mp_dir.to_string_lossy().into_owned(),
        )
        .await?;

        apply_server_defaults(&tes3mp_dir, &new_instance)?;

        Ok::<(), String>(())
    })
    .await
    {
        cleanup_failed_instance_root(instance_root);
        return Err(err);
    }

    let instance = build_instance_config(&new_instance);
    let (config, app_handle) = persist_owned_instance_to_config(&state, instance)?;

    app_handle
        .emit("on_config_added_instance", config.clone())
        .map_err(|e| e.to_string())?;

    app_handle
        .emit("on_config_change", config)
        .map_err(|e| e.to_string())?;

    info!(
        "Instance '{}' created at {}",
        new_instance.instance_name, new_instance.instance_root_path
    );

    Ok(())
}

// pub async fn download_and_run_openmw_wizard(
//     state: State<'_, Mutex<AppState>>,
// ) -> Result<(), String> {
//     // Use const release id to call github_getters::download_and_extract_release_zip_by_id_to_path
//     // Use the root path + "Base TES3MP" to create the path for the extracted files
//     // once extracted run openmw-wizard.exe
//     // once openmw-wizard.exe is done, return Ok(())
//     // if any error occurs, return Err(String::from("Failed to download and run openmw-wizard"))
//     let root_path = state
//         .lock()
//         .unwrap()
//         .nerevar_config
//         .root_path
//         .clone()
//         .ok_or_else(|| "Root path not set".to_string())?;
//     let base_tes3mp_path = Path::new(&root_path).join("Base TES3MP");
//     std::fs::create_dir_all(&base_tes3mp_path).map_err(|e| e.to_string())?;
//     github_getters::download_and_extract_release_zip_by_id_to_path(
//         TES3MP_081_RELEASE_ID.to_string(),
//         base_tes3mp_path.to_string_lossy().into_owned(),
//     )
//     .await?;
//     let openmw_wizard_path = base_tes3mp_path.join("openmw-wizard.exe");
//     if !openmw_wizard_path.exists() {
//         return Err(String::from("Failed to download and run openmw-wizard"));
//     }
//     let mut result = std::process::Command::new(openmw_wizard_path)
//         .spawn()
//         .map_err(|e| e.to_string())?;
//     let status = result.wait().map_err(|e| e.to_string())?;
//     if !status.success() {
//         return Err(String::from("Failed to run openmw-wizard"));
//     }
//     Ok(())
// }

pub async fn validate_global_openmw_config() -> Result<bool, String> {
    crate::openmw_ini_importer::validate_nerevar_openmw_scaffold()
}

pub async fn generate_default_global_openmw_config(
    morrowind_installation_path: String,
) -> Result<(), String> {
    crate::openmw_ini_importer::setup_nerevar_openmw_scaffold(Path::new(
        &morrowind_installation_path,
    ))
}
