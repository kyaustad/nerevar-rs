use std::path::Path;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, State};

use crate::data::{InstanceConfig, NerevarConfig};
use crate::instance_data::{find_instance_by_id, resolve_package_data_dir};
use crate::process_manager::{ProcessManager, ProcessRole};
use crate::sync_client::SyncCoordinator;
use crate::sync_host::SharedSyncHost;
use crate::AppState;

fn instance_is_running(
    process_manager: &ProcessManager,
    instance_id: &str,
) -> Result<Option<ProcessRole>, String> {
    if process_manager.is_running(instance_id, ProcessRole::Client)? {
        return Ok(Some(ProcessRole::Client));
    }
    if process_manager.is_running(instance_id, ProcessRole::Server)? {
        return Ok(Some(ProcessRole::Server));
    }
    Ok(None)
}

fn clear_hosting_if_needed(
    app: &AppHandle,
    sync_host: &SharedSyncHost,
    instance_id: &str,
) -> Result<(), String> {
    let mut host = sync_host
        .lock()
        .map_err(|_| "Sync host lock poisoned".to_string())?;

    if host
        .hosting_instance_id
        .as_deref()
        .is_some_and(|id| id == instance_id)
    {
        host.hosting_instance_id = None;
        host.hosting_data_dir = None;
        host.hosting_instance_root = None;
        let _ = app.emit("hosting-changed", ());
    }

    Ok(())
}

fn delete_instance_files(instance: &InstanceConfig) -> Result<(), String> {
    let root = Path::new(&instance.path);
    if root.exists() {
        std::fs::remove_dir_all(root).map_err(|error| {
            format!(
                "Failed to delete instance folder at {}: {error}",
                root.display()
            )
        })?;
        return Ok(());
    }

    let data_dir = resolve_package_data_dir(instance);
    if data_dir.exists() {
        std::fs::remove_dir_all(&data_dir).map_err(|error| {
            format!(
                "Failed to delete data directory at {}: {error}",
                data_dir.display()
            )
        })?;
    }

    Ok(())
}

fn remove_instance_from_config(
    guard: &mut AppState,
    instance_id: &str,
) -> Result<(), String> {
    if let Some(owned) = guard.nerevar_config.owned_instances.as_mut() {
        if let Some(index) = owned.iter().position(|instance| instance.id == instance_id) {
            owned.remove(index);
            if owned.is_empty() {
                guard.nerevar_config.owned_instances = None;
            }
            return Ok(());
        }
    }

    if let Some(synced) = guard.nerevar_config.synced_instances.as_mut() {
        if let Some(index) = synced.iter().position(|instance| instance.id == instance_id) {
            synced.remove(index);
            if synced.is_empty() {
                guard.nerevar_config.synced_instances = None;
            }
            return Ok(());
        }
    }

    Err(format!("Instance not found: {instance_id}"))
}

#[tauri::command]
pub fn delete_instance(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    sync_host: State<'_, SharedSyncHost>,
    process_manager: State<'_, Arc<ProcessManager>>,
    coordinator: State<'_, Arc<SyncCoordinator>>,
    instance_id: String,
    delete_data_directory: bool,
) -> Result<NerevarConfig, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    if let Some(role) = instance_is_running(process_manager.inner(), &instance_id)? {
        return Err(format!(
            "Cannot delete instance while its TES3MP {} is running. Stop it first.",
            role.as_str()
        ));
    }

    let _ = coordinator.cancel(&instance_id);
    clear_hosting_if_needed(&app, sync_host.inner(), &instance_id)?;

    if delete_data_directory {
        delete_instance_files(&instance)?;
    }

    let config = {
        let mut guard = state
            .lock()
            .map_err(|_| "App state lock poisoned".to_string())?;
        remove_instance_from_config(&mut guard, &instance_id)?;
        std::fs::write(
            Path::new(&guard.nerevar_config_path),
            serde_json::to_string_pretty(&guard.nerevar_config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        guard.nerevar_config.clone()
    };

    let _ = app.emit("on_config_change", config.clone());
    Ok(config)
}
