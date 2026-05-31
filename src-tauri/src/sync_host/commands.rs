use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use crate::data::NerevarConfig;
use crate::instance_data::{
    find_instance_by_id, load_manifest, manifest_path, resolve_package_data_dir,
    NerevarManifest,
};
use crate::sync_client::ping_nerevar_server;
use crate::AppState;

use super::status::SyncHostStatus;
use super::SharedSyncHost;

fn instance_name(config: &NerevarConfig, instance_id: &str) -> Option<String> {
    find_instance_by_id(config, instance_id).map(|instance| instance.name.clone())
}

fn build_status(
    config: &NerevarConfig,
    hosting_instance_id: Option<String>,
    hosting_data_dir: Option<&Path>,
    server_online: bool,
) -> SyncHostStatus {
    let manifest_available = hosting_data_dir
        .map(manifest_path)
        .is_some_and(|path| path.is_file());

    let hosting_instance_name = hosting_instance_id
        .as_ref()
        .and_then(|id| instance_name(config, id));

    SyncHostStatus {
        sync_port: config.sync_port as u32,
        server_online,
        hosting_instance_id,
        hosting_instance_name,
        manifest_available,
    }
}

fn emit_hosting_changed(app: &AppHandle) {
    let _ = app.emit("hosting-changed", ());
}

#[tauri::command]
pub async fn get_sync_host_status(
    state: State<'_, Mutex<AppState>>,
    sync_host: State<'_, SharedSyncHost>,
) -> Result<SyncHostStatus, String> {
    let (config, hosting_instance_id, hosting_data_dir) = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        let host = sync_host
            .lock()
            .map_err(|_| "Sync host lock poisoned".to_string())?;
        (
            guard.nerevar_config.clone(),
            host.hosting_instance_id.clone(),
            host.hosting_data_dir.clone(),
        )
    };

    let port = config.sync_port as u16;
    let server_online = ping_nerevar_server("127.0.0.1", port).await.is_ok();

    Ok(build_status(
        &config,
        hosting_instance_id,
        hosting_data_dir.as_deref(),
        server_online,
    ))
}

/// Start hosting using the manifest already on disk (does not rebuild).
#[tauri::command]
pub fn activate_hosting_instance(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    sync_host: State<'_, SharedSyncHost>,
    instance_id: String,
) -> Result<NerevarManifest, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    let data_dir = resolve_package_data_dir(&instance);
    let manifest = load_manifest(&data_dir).map_err(|_| {
        format!(
            "No manifest found at {}. Open the data manager, save your load order, and use \"Save & host manifest\" first.",
            manifest_path(&data_dir).display()
        )
    })?;

    let mut host = sync_host
        .lock()
        .map_err(|_| "Sync host lock poisoned".to_string())?;
    host.hosting_instance_id = Some(instance_id);
    host.hosting_data_dir = Some(data_dir);
    host.hosting_instance_root = Some(Path::new(&instance.path).to_path_buf());

    emit_hosting_changed(&app);
    Ok(manifest)
}

#[tauri::command]
pub fn clear_hosting_instance(
    app: AppHandle,
    sync_host: State<'_, SharedSyncHost>,
) -> Result<(), String> {
    let mut host = sync_host
        .lock()
        .map_err(|_| "Sync host lock poisoned".to_string())?;
    host.hosting_instance_id = None;
    host.hosting_data_dir = None;
    host.hosting_instance_root = None;
    emit_hosting_changed(&app);
    Ok(())
}
