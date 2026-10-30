use std::path::Path;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, State};

use crate::config::nerevar_config::{
    build_synced_instance_config, persist_synced_instance_to_config, update_synced_instance,
};
use crate::data::NewConnectionConfig;
use crate::instance_data::find_instance_by_id;
use crate::instance_data::{
    load_manifest, resolve_package_data_dir, ManifestValidationResult,
};
use crate::instance_setup::{
    create_instance_data_dir, instance_tes3mp_dir, write_owned_client_connection,
    write_tes3mp_client_connection,
};
use crate::instance_data::ensure_instance_data_layout;
use crate::github_getters;
use crate::process_manager::{
    launch_tes3mp_client, launch_tes3mp_server, stop_tes3mp_process, GlobalProcessStatus,
    ProcessManager, ProcessRole,
};
use crate::sync_client::{
    fetch_manifest_summary, ping_nerevar_server, run_instance_sync, sync_if_needed,
    touch_last_synced, write_synced_client_connection, RemoteManifestSummary, SyncCoordinator,
};
use crate::AppState;

#[tauri::command]
pub async fn ping_remote_nerevar_server(
    remote_host: String,
    remote_sync_port: u16,
) -> Result<(), String> {
    ping_nerevar_server(&remote_host, remote_sync_port).await
}

#[tauri::command]
pub async fn fetch_remote_manifest_summary(
    remote_host: String,
    remote_sync_port: u16,
    sync_password: Option<String>,
) -> Result<RemoteManifestSummary, String> {
    fetch_manifest_summary(
        &remote_host,
        remote_sync_port,
        sync_password.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn add_synced_connection(
    state: State<'_, Mutex<AppState>>,
    _app: AppHandle,
    new_connection: NewConnectionConfig,
) -> Result<String, String> {
    ping_nerevar_server(&new_connection.remote_host, new_connection.remote_sync_port).await?;
    let summary = fetch_manifest_summary(
        &new_connection.remote_host,
        new_connection.remote_sync_port,
        Some(new_connection.sync_password.as_str()),
    )
    .await?;

    // TES3MP-NG clients must run the host's build
    let tag = summary.tes3mp_release_tag.clone().ok_or_else(|| {
        "The host didn't report its TES3MP-NG release. The host needs to update Nerevar and host the instance again."
            .to_string()
    })?;
    let release = github_getters::find_release_by_tag(&tag).await?;

    let root_path = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        guard.nerevar_config.root_path.clone()
    }
    .ok_or_else(|| "Choose where Nerevar stores instances first (Settings).".to_string())?;
    let instance_root = unique_instance_root(Path::new(&root_path), &summary.instance_name);
    let instance_data_dir = instance_root.join(crate::instance_data::INSTANCE_DATA_DIR);

    if let Err(err) = (async {
        std::fs::create_dir_all(&instance_root).map_err(|e| e.to_string())?;
        create_instance_data_dir(&instance_data_dir)?;
        ensure_instance_data_layout(&instance_data_dir)?;

        let tes3mp_dir = instance_tes3mp_dir(&instance_root);
        github_getters::install_release(&release, &tes3mp_dir).await?;

        // The game server's password comes with the first sync
        write_tes3mp_client_connection(
            &tes3mp_dir,
            &new_connection.remote_host,
            summary.tes3mp_server_port,
            "",
        )?;

        Ok::<(), String>(())
    })
    .await
    {
        if instance_root.exists() {
            let _ = std::fs::remove_dir_all(&instance_root);
        }
        return Err(err);
    }

    let mut instance = build_synced_instance_config(
        &new_connection,
        &summary,
        &release,
        &instance_root,
        &instance_data_dir,
    );
    instance.tes3mp_server_port = Some(summary.tes3mp_server_port);
    let instance_id = instance.id.clone();
    let (config, app_handle) = persist_synced_instance_to_config(&state, instance)?;

    app_handle
        .emit("on_config_added_connection", config.clone())
        .map_err(|e| e.to_string())?;
    app_handle
        .emit("on_config_change", config)
        .map_err(|e| e.to_string())?;

    Ok(instance_id)
}

/// A new folder under the Nerevar root named after the host's instance, with " (2)", " (3)"...
/// when it's taken.
fn unique_instance_root(root: &Path, instance_name: &str) -> std::path::PathBuf {
    let name: String = instance_name
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') { c } else { '_' })
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .to_string();
    let name = if name.is_empty() { "Synced instance".to_string() } else { name };

    let mut candidate = root.join(&name);
    let mut number = 2;
    while candidate.exists() {
        candidate = root.join(format!("{name} ({number})"));
        number += 1;
    }
    candidate
}

#[tauri::command]
pub async fn sync_instance_from_remote(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    coordinator: State<'_, Arc<SyncCoordinator>>,
    instance_id: String,
) -> Result<ManifestValidationResult, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    let validation =
        run_instance_sync(app.clone(), coordinator.inner().clone(), &instance).await?;

    if validation.valid {
        let data_dir = resolve_package_data_dir(&instance);
        let manifest = load_manifest(&data_dir)?;
        let mut updated = instance;
        touch_last_synced(&mut updated, &manifest);
        let config = update_synced_instance(&state, updated)?;
        let _ = app.emit("on_config_change", config);
    }

    Ok(validation)
}

#[tauri::command]
pub fn cancel_instance_sync(
    coordinator: State<'_, Arc<SyncCoordinator>>,
    instance_id: String,
) -> Result<bool, String> {
    Ok(coordinator.cancel(&instance_id))
}

#[tauri::command]
pub async fn launch_instance_client(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    process_manager: State<'_, Arc<ProcessManager>>,
    coordinator: State<'_, Arc<SyncCoordinator>>,
    instance_id: String,
) -> Result<(), String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    if instance.remote_host.is_some() {
        let validation = sync_if_needed(
            app.clone(),
            coordinator.inner().clone(),
            &instance,
            false,
        )
        .await?;

        if !validation.valid {
            return Err(format!(
                "Cannot launch: file validation failed ({} issue(s))",
                validation.issues.len()
            ));
        }

        let data_dir = resolve_package_data_dir(&instance);
        let manifest = load_manifest(&data_dir)?;
        write_synced_client_connection(&instance, &manifest)?;

        let mut updated = instance.clone();
        touch_last_synced(&mut updated, &manifest);
        if updated.tes3mp_server_port != instance.tes3mp_server_port
            || updated.last_synced_at != instance.last_synced_at
        {
            let config = update_synced_instance(&state, updated)?;
            let _ = app.emit("on_config_change", config);
        }
    } else {
        let tes3mp_dir = instance_tes3mp_dir(Path::new(&instance.path));
        write_owned_client_connection(&tes3mp_dir)?;
    }

    process_manager
        .inner()
        .ensure_can_launch(&instance_id, ProcessRole::Client)?;

    let data_dir = resolve_package_data_dir(&instance);
    // Synced instances get the host's client files (shader settings...)
    let client_files = if instance.remote_host.is_some() {
        load_manifest(&data_dir)?.client_files
    } else {
        Vec::new()
    };
    launch_tes3mp_client(
        app,
        process_manager.inner().clone(),
        &instance_id,
        Path::new(&instance.path),
        &data_dir,
        &client_files,
    )
}

#[tauri::command]
pub fn launch_instance_server(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    process_manager: State<'_, Arc<ProcessManager>>,
    instance_id: String,
) -> Result<(), String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    process_manager
        .inner()
        .ensure_can_launch(&instance_id, ProcessRole::Server)?;

    let data_dir = resolve_package_data_dir(&instance);
    launch_tes3mp_server(
        app,
        process_manager.inner().clone(),
        &instance_id,
        Path::new(&instance.path),
        &data_dir,
    )
}

#[tauri::command]
pub fn stop_instance_process(
    app: AppHandle,
    process_manager: State<'_, Arc<ProcessManager>>,
    instance_id: String,
    role: String,
) -> Result<bool, String> {
    let role = ProcessRole::from_str(&role).ok_or_else(|| format!("Invalid process role: {role}"))?;
    stop_tes3mp_process(app, process_manager.inner(), &instance_id, role)
}

#[tauri::command]
pub fn is_instance_process_running(
    process_manager: State<'_, Arc<ProcessManager>>,
    instance_id: String,
    role: String,
) -> Result<bool, String> {
    let role = ProcessRole::from_str(&role).ok_or_else(|| format!("Invalid process role: {role}"))?;
    process_manager.is_running(&instance_id, role)
}

#[tauri::command]
pub fn get_global_process_status(
    process_manager: State<'_, Arc<ProcessManager>>,
) -> Result<GlobalProcessStatus, String> {
    process_manager.global_status()
}
