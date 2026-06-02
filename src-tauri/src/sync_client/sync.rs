use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::data::InstanceConfig;
use crate::instance_data::{
    find_instance_by_id, load_load_order, load_manifest, manifest_path, manifests_differ,
    resolve_package_data_dir, resolve_synced_load_order, validate_manifest_against_disk,
    write_instance_launch_cfg, ManifestValidationResult, NerevarManifest, ResolvedOpenMwConfig,
};
use crate::AppState;

use super::apply::apply_manifest_to_load_order;
use super::coordinator::SyncCoordinator;
use super::download::{download_manifest_files, persist_manifest, DownloadOutcome};
use super::fetch::fetch_full_manifest;
use super::metadata::{apply_manifest_metadata, write_synced_client_connection};
use super::progress::emit_sync_progress;
use super::sync_state::{
    instance_sync_status, instance_sync_status_absent, load_sync_state, sync_is_complete,
};
use super::types::{InstanceSyncStatus, SyncPhase, SyncProgressEvent};

pub async fn run_instance_sync(
    app: AppHandle,
    coordinator: Arc<SyncCoordinator>,
    instance: &InstanceConfig,
) -> Result<ManifestValidationResult, String> {
    sync_if_needed(app, coordinator, instance, false).await
}

/// When `force` is true, re-downloads every file. Otherwise resumes using sync-state checksums.
pub async fn sync_if_needed(
    app: AppHandle,
    coordinator: Arc<SyncCoordinator>,
    instance: &InstanceConfig,
    force: bool,
) -> Result<ManifestValidationResult, String> {
    let host = instance
        .remote_host
        .as_deref()
        .ok_or_else(|| "Instance has no remote host configured".to_string())?;
    let port = instance
        .remote_sync_port
        .ok_or_else(|| "Instance has no remote sync port configured".to_string())?;
    let sync_password = instance.sync_password.as_deref();

    let instance_id = instance.id.clone();
    let data_dir = resolve_package_data_dir(instance);
    let cancel = coordinator.begin(&instance_id)?;

    let result = sync_if_needed_inner(
        app.clone(),
        instance,
        &instance_id,
        host,
        port,
        sync_password,
        &data_dir,
        cancel.clone(),
        force,
    )
    .await;

    if cancel.load(Ordering::Relaxed) {
        coordinator.finish(&instance_id);
        return result;
    }

    coordinator.finish(&instance_id);
    result
}

#[tauri::command]
pub fn get_instance_sync_status(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<InstanceSyncStatus, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };

    let data_dir = resolve_package_data_dir(&instance);
    if !manifest_path(&data_dir).exists() {
        return Ok(instance_sync_status_absent());
    }

    let manifest = load_manifest(&data_dir)?;
    instance_sync_status(&data_dir, &manifest)
}

async fn sync_if_needed_inner(
    app: AppHandle,
    instance: &InstanceConfig,
    instance_id: &str,
    host: &str,
    port: u16,
    sync_password: Option<&str>,
    data_dir: &Path,
    cancel: Arc<std::sync::atomic::AtomicBool>,
    force: bool,
) -> Result<ManifestValidationResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Sync cancelled".to_string());
    }

    emit_sync_progress(
        &app,
        instance_id,
        SyncPhase::CheckingUpdates,
        "Checking host manifest for updates",
        0,
        1,
        0,
        0,
        None,
    );
    let remote = fetch_full_manifest(host, port, sync_password).await?;

    let local_manifest = if manifest_path(data_dir).exists() {
        Some(load_manifest(data_dir)?)
    } else {
        None
    };

    let needs_download = if force {
        true
    } else if !manifest_path(data_dir).exists() {
        true
    } else {
        let local = load_manifest(data_dir)?;
        if manifests_differ(&local, &remote) {
            true
        } else {
            let state = load_sync_state(data_dir, &remote)?;
            !sync_is_complete(&state, &remote)
        }
    };

    if !needs_download {
        emit_sync_progress(
            &app,
            instance_id,
            SyncPhase::Complete,
            "Already up to date",
            1,
            1,
            0,
            0,
            None,
        );
        let local = load_manifest(data_dir)?;
        finalize_after_sync(instance, data_dir, &local)?;
        return Ok(ManifestValidationResult {
            valid: true,
            issues: Vec::new(),
        });
    }

    if !force {
        if let Ok(status) = instance_sync_status(data_dir, &remote) {
            if status.can_resume {
                emit_sync_progress(
                    &app,
                    instance_id,
                    SyncPhase::CheckingUpdates,
                    format!(
                        "Resuming previous sync ({}% complete)",
                        status.percent_complete
                    ),
                    status.bytes_verified,
                    status.bytes_total.max(1),
                    status.files_verified as u64,
                    status.files_total as u64,
                    None,
                );
            }
        }
    }

    emit_sync_progress(
        &app,
        instance_id,
        SyncPhase::FetchingManifest,
        "Fetching manifest from host",
        0,
        1,
        0,
        0,
        None,
    );

    persist_manifest(data_dir, &remote)?;

    let download_result = download_manifest_files(
        app.clone(),
        instance_id,
        host,
        port,
        sync_password,
        data_dir,
        &remote,
        force,
        cancel.clone(),
    )
    .await?;

    match download_result {
        DownloadOutcome::Complete { bytes_done } => {
            emit_sync_progress(
                &app,
                instance_id,
                SyncPhase::ApplyingLoadOrder,
                "Removing deleted mods and files",
                bytes_done,
                remote.total_download_bytes.max(1),
                0,
                0,
                None,
            );
        }
        DownloadOutcome::Cancelled {
            bytes_done,
            bytes_total,
        } => {
            let status = instance_sync_status(data_dir, &remote).unwrap_or_else(|_| {
                InstanceSyncStatus {
                    has_manifest: true,
                    can_resume: true,
                    is_complete: false,
                    bytes_verified: bytes_done,
                    bytes_total,
                    files_verified: 0,
                    files_total: 0,
                    percent_complete: if bytes_total > 0 {
                        ((bytes_done * 100) / bytes_total).min(100) as u8
                    } else {
                        0
                    },
                }
            });
            let _ = app.emit(
                "sync-progress",
                SyncProgressEvent {
                    instance_id: instance_id.to_string(),
                    phase: SyncPhase::Cancelled,
                    message: format!(
                        "Sync paused at {}% — run Sync again to resume",
                        status.percent_complete
                    ),
                    bytes_done,
                    bytes_total,
                    files_done: status.files_verified as u64,
                    files_total: status.files_total as u64,
                    overall_percent: status.percent_complete,
                    current_file: None,
                },
            );
            return Err("Sync cancelled".to_string());
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return Err("Sync cancelled".to_string());
    }

    crate::instance_data::prune_local_against_manifest(data_dir, &remote)?;

    if cancel.load(Ordering::Relaxed) {
        return Err("Sync cancelled".to_string());
    }

    emit_sync_progress(
        &app,
        instance_id,
        SyncPhase::Validating,
        "Verifying downloaded files",
        0,
        1,
        0,
        0,
        None,
    );
    let validation = validate_manifest_against_disk(data_dir, &remote);
    if !validation.valid {
        let message = format!("Validation failed ({} issues)", validation.issues.len());
        emit_sync_progress(
            &app,
            instance_id,
            SyncPhase::Failed,
            message,
            0,
            1,
            0,
            0,
            None,
        );
        return Ok(validation);
    }

    finalize_after_sync(instance, data_dir, &remote)?;
    emit_sync_progress(
        &app,
        instance_id,
        SyncPhase::Complete,
        "Sync complete",
        remote.total_download_bytes.max(1),
        remote.total_download_bytes.max(1),
        0,
        0,
        None,
    );
    Ok(validation)
}

fn finalize_after_sync(
    instance: &InstanceConfig,
    data_dir: &Path,
    manifest: &NerevarManifest,
) -> Result<(), String> {
    apply_manifest_to_load_order(data_dir, manifest)?;
    let load_order = load_load_order(data_dir)?;
    let resolved: ResolvedOpenMwConfig =
        resolve_synced_load_order(data_dir, &load_order, manifest)?;
    write_instance_launch_cfg(
        data_dir,
        &resolved,
        &manifest.instance_settings.openmw_cfg_overrides,
    )?;
    write_synced_client_connection(instance, manifest)?;
    crate::instance_settings::persist_settings_from_manifest(data_dir, &manifest.instance_settings)?;
    Ok(())
}

pub fn touch_last_synced(instance: &mut InstanceConfig, manifest: &NerevarManifest) {
    instance.last_synced_at = Some(Utc::now().to_rfc3339());
    apply_manifest_metadata(instance, manifest);
}
