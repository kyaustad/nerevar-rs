use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use reqwest::Client;
use tauri::{AppHandle, Emitter};

use crate::instance_data::{package_abs_path, NerevarManifest};
use crate::sync_auth::SYNC_PASSWORD_HEADER;

use super::types::{SyncPhase, SyncProgressEvent};

fn base_url(host: &str, port: u16) -> String {
    let host = host.trim().trim_end_matches('/');
    format!("http://{host}:{port}")
}

fn emit_progress(
    app: &AppHandle,
    instance_id: &str,
    phase: SyncPhase,
    message: impl Into<String>,
    bytes_done: u64,
    bytes_total: u64,
    current_file: Option<String>,
) {
    let _ = app.emit(
        "sync-progress",
        SyncProgressEvent {
            instance_id: instance_id.to_string(),
            phase,
            message: message.into(),
            bytes_done,
            bytes_total,
            current_file,
        },
    );
}

pub async fn download_manifest_files(
    app: AppHandle,
    instance_id: &str,
    host: &str,
    port: u16,
    sync_password: Option<&str>,
    data_dir: &Path,
    manifest: &NerevarManifest,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let client = Client::new();
    let bytes_total = manifest.total_download_bytes.max(1);
    let mut bytes_done = 0u64;

    emit_progress(
        &app,
        instance_id,
        SyncPhase::Downloading,
        "Starting download",
        bytes_done,
        bytes_total,
        None,
    );

    for package in &manifest.packages {
        if cancel.load(Ordering::Relaxed) {
            return Err("Sync cancelled".to_string());
        }

        let package_dir = package_abs_path(data_dir, &package.relative_dir);
        std::fs::create_dir_all(&package_dir)
            .map_err(|e| format!("Failed to create {}: {e}", package_dir.display()))?;

        for file_entry in &package.files {
            if cancel.load(Ordering::Relaxed) {
                return Err("Sync cancelled".to_string());
            }

            let dest = package_dir.join(&file_entry.path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            let url = format!(
                "{}/packages/{}/files/{}",
                base_url(host, port),
                urlencoding::encode(&package.id),
                encode_path_segments(&file_entry.path)
            );

            emit_progress(
                &app,
                instance_id,
                SyncPhase::Downloading,
                format!("Downloading {}", file_entry.path),
                bytes_done,
                bytes_total,
                Some(file_entry.path.clone()),
            );

            let mut request = client
                .get(&url)
                .header("User-Agent", "Nerevar-0.1.0");
            if let Some(password) = sync_password.filter(|value| !value.is_empty()) {
                request = request.header(SYNC_PASSWORD_HEADER, password);
            }
            let response = request
                .send()
                .await
                .map_err(|e| format!("Failed to download {}: {e}", file_entry.path))?;

            if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                return Err("Sync password required or incorrect".to_string());
            }

            if !response.status().is_success() {
                return Err(format!(
                    "Failed to download {} (HTTP {})",
                    file_entry.path,
                    response.status()
                ));
            }

            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Failed to read {}: {e}", file_entry.path))?;

            std::fs::write(&dest, &bytes).map_err(|e| {
                format!(
                    "Failed to write {}: {e}",
                    dest.to_string_lossy()
                )
            })?;

            bytes_done = bytes_done.saturating_add(bytes.len() as u64);
            emit_progress(
                &app,
                instance_id,
                SyncPhase::Downloading,
                format!("Downloaded {}", file_entry.path),
                bytes_done,
                bytes_total,
                Some(file_entry.path.clone()),
            );
        }
    }

    Ok(())
}

fn encode_path_segments(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .map(urlencoding::encode)
        .map(|s| s.into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn persist_manifest(data_dir: &Path, manifest: &NerevarManifest) -> Result<PathBuf, String> {
    let path = crate::instance_data::manifest_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write manifest: {e}"))?;
    Ok(path)
}
