use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::data::InstanceConfig;
use crate::instance_data::{
    load_load_order, load_manifest, manifest_path, manifests_differ, resolve_package_data_dir,
    resolve_synced_load_order, validate_manifest_against_disk, write_instance_launch_cfg,
    ManifestValidationResult, NerevarManifest, ResolvedOpenMwConfig,
};

use super::apply::apply_manifest_to_load_order;
use super::coordinator::SyncCoordinator;
use super::download::{download_manifest_files, persist_manifest};
use super::fetch::fetch_full_manifest;
use super::metadata::{apply_manifest_metadata, write_synced_client_connection};
use super::types::{SyncPhase, SyncProgressEvent};

pub async fn run_instance_sync(
    app: AppHandle,
    coordinator: Arc<SyncCoordinator>,
    instance: &InstanceConfig,
) -> Result<ManifestValidationResult, String> {
    sync_if_needed(app, coordinator, instance, true).await
}

/// Returns validation after ensuring local files match the remote manifest.
/// When `force` is false, skips download if the manifest is unchanged and local files validate.
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
        let _ = app.emit(
            "sync-progress",
            SyncProgressEvent {
                instance_id: instance_id.clone(),
                phase: SyncPhase::Cancelled,
                message: "Sync cancelled".to_string(),
                bytes_done: 0,
                bytes_total: 0,
                current_file: None,
            },
        );
        coordinator.finish(&instance_id);
        return Err("Sync cancelled".to_string());
    }

    coordinator.finish(&instance_id);
    result
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
    let emit = |phase: SyncPhase, message: &str, bytes_done: u64, bytes_total: u64| {
        let _ = app.emit(
            "sync-progress",
            SyncProgressEvent {
                instance_id: instance_id.to_string(),
                phase,
                message: message.to_string(),
                bytes_done,
                bytes_total,
                current_file: None,
            },
        );
    };

    if cancel.load(Ordering::Relaxed) {
        return Err("Sync cancelled".to_string());
    }

    emit(
        SyncPhase::CheckingUpdates,
        "Checking host manifest for updates",
        0,
        1,
    );
    let remote = fetch_full_manifest(host, port, sync_password).await?;

    let needs_download = if force {
        true
    } else if !manifest_path(data_dir).exists() {
        true
    } else {
        let local = load_manifest(data_dir)?;
        if manifests_differ(&local, &remote) {
            true
        } else {
            let validation = validate_manifest_against_disk(data_dir, &local);
            !validation.valid
        }
    };

    if !needs_download {
        emit(
            SyncPhase::Complete,
            "Already up to date",
            1,
            1,
        );
        let local = load_manifest(data_dir)?;
        finalize_after_sync(instance, data_dir, &local)?;
        return Ok(ManifestValidationResult {
            valid: true,
            issues: Vec::new(),
        });
    }

    emit(
        SyncPhase::FetchingManifest,
        "Fetching manifest from host",
        0,
        1,
    );
    persist_manifest(data_dir, &remote)?;

    download_manifest_files(
        app.clone(),
        instance_id,
        host,
        port,
        sync_password,
        data_dir,
        &remote,
        cancel.clone(),
    )
    .await?;

    if cancel.load(Ordering::Relaxed) {
        return Err("Sync cancelled".to_string());
    }

    emit(
        SyncPhase::ApplyingLoadOrder,
        "Removing deleted mods and files",
        0,
        1,
    );
    crate::instance_data::prune_local_against_manifest(data_dir, &remote)?;

    if cancel.load(Ordering::Relaxed) {
        return Err("Sync cancelled".to_string());
    }

    emit(SyncPhase::Validating, "Verifying downloaded files", 0, 1);
    let validation = validate_manifest_against_disk(data_dir, &remote);
    if !validation.valid {
        let message = format!("Validation failed ({} issues)", validation.issues.len());
        emit(SyncPhase::Failed, &message, 0, 1);
        return Ok(validation);
    }

    finalize_after_sync(instance, data_dir, &remote)?;
    emit(SyncPhase::Complete, "Sync complete", 1, 1);
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
    write_instance_launch_cfg(data_dir, &resolved)?;
    write_synced_client_connection(instance, manifest)?;
    crate::instance_settings::persist_settings_from_manifest(data_dir, &manifest.instance_settings)?;
    Ok(())
}

pub fn touch_last_synced(instance: &mut InstanceConfig, manifest: &NerevarManifest) {
    instance.last_synced_at = Some(Utc::now().to_rfc3339());
    apply_manifest_metadata(instance, manifest);
}
