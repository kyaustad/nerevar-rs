use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::Client;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;

use crate::instance_data::{package_abs_path, ManifestFileEntry, NerevarManifest};
use crate::sync_auth::SYNC_PASSWORD_HEADER;

use super::sync_state::{
    adopt_existing_files_into_state, clear_sync_state, hash_file_checksum, SharedSyncState,
};
use super::progress::emit_sync_progress;
use super::types::SyncPhase;

const MAX_CONCURRENT_DOWNLOADS: usize = 16;
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(250);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

fn user_agent() -> String {
    format!("Nerevar-{}", env!("CARGO_PKG_VERSION"))
}

fn sync_download_client() -> Result<Client, String> {
    Client::builder()
        .pool_max_idle_per_host(MAX_CONCURRENT_DOWNLOADS)
        .tcp_nodelay(true)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()
        .map_err(|e| format!("Failed to create download client: {e}"))
}

pub enum DownloadOutcome {
    Complete { bytes_done: u64 },
    Cancelled { bytes_done: u64, bytes_total: u64 },
}

struct ProgressThrottler {
    last_emit: Mutex<Instant>,
}

impl ProgressThrottler {
    fn new() -> Self {
        Self {
            last_emit: Mutex::new(Instant::now() - PROGRESS_EMIT_INTERVAL),
        }
    }

    fn should_emit(&self) -> bool {
        let Ok(mut last_emit) = self.last_emit.lock() else {
            return true;
        };
        if last_emit.elapsed() >= PROGRESS_EMIT_INTERVAL {
            *last_emit = Instant::now();
            true
        } else {
            false
        }
    }
}

fn maybe_emit_progress(
    throttler: &ProgressThrottler,
    app: &AppHandle,
    instance_id: &str,
    phase: SyncPhase,
    message: impl Into<String>,
    bytes_done: u64,
    bytes_total: u64,
    files_done: u64,
    files_total: u64,
    current_file: Option<String>,
    force: bool,
) {
    if force || throttler.should_emit() {
        emit_sync_progress(
            app,
            instance_id,
            phase,
            message,
            bytes_done,
            bytes_total,
            files_done,
            files_total,
            current_file,
        );
    }
}

fn base_url(host: &str, port: u16) -> String {
    let host = host.trim().trim_end_matches('/');
    format!("http://{host}:{port}")
}

struct DownloadJob {
    package_id: String,
    file_entry: ManifestFileEntry,
    dest: PathBuf,
    url: String,
}

struct JobQueue {
    jobs: Vec<DownloadJob>,
    next: AtomicUsize,
}

impl JobQueue {
    fn new(jobs: Vec<DownloadJob>) -> Self {
        Self {
            jobs,
            next: AtomicUsize::new(0),
        }
    }

    fn pop(&self) -> Option<&DownloadJob> {
        let index = self.next.fetch_add(1, Ordering::Relaxed);
        self.jobs.get(index)
    }
}

fn part_path(dest: &Path) -> PathBuf {
    dest.with_file_name(format!(
        "{}.nerevar-part",
        dest.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("download")
    ))
}

fn count_verified_bytes(manifest: &NerevarManifest, state: &SharedSyncState) -> u64 {
    let mut bytes = 0u64;
    for package in &manifest.packages {
        for file in &package.files {
            if state.is_verified(&package.id, file) {
                bytes = bytes.saturating_add(file.size);
            }
        }
    }
    bytes
}

fn collect_download_jobs(
    data_dir: &Path,
    manifest: &NerevarManifest,
    host: &str,
    port: u16,
    force: bool,
    state: &SharedSyncState,
) -> Result<Vec<DownloadJob>, String> {
    let base = base_url(host, port);
    let mut jobs = Vec::new();

    for package in &manifest.packages {
        let package_dir = package_abs_path(data_dir, &package.relative_dir);
        std::fs::create_dir_all(&package_dir)
            .map_err(|e| format!("Failed to create {}: {e}", package_dir.display()))?;

        for file_entry in &package.files {
            if !force && state.is_verified(&package.id, file_entry) {
                continue;
            }

            let dest = package_dir.join(&file_entry.path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            let _ = std::fs::remove_file(part_path(&dest));

            let url = format!(
                "{}/packages/{}/files/{}",
                base,
                urlencoding::encode(&package.id),
                encode_path_segments(&file_entry.path)
            );

            jobs.push(DownloadJob {
                package_id: package.id.clone(),
                file_entry: file_entry.clone(),
                dest,
                url,
            });
        }
    }

    Ok(jobs)
}

async fn download_one_file(
    client: &Client,
    sync_password: Option<&str>,
    job: &DownloadJob,
    state: &SharedSyncState,
    cancel: &AtomicBool,
) -> Result<u64, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Sync cancelled".to_string());
    }

    let mut request = client.get(&job.url).header("User-Agent", user_agent());
    if let Some(password) = sync_password.filter(|value| !value.is_empty()) {
        request = request.header(SYNC_PASSWORD_HEADER, password);
    }

    let mut response = request
        .send()
        .await
        .map_err(|e| format!("Failed to download {}: {e}", job.file_entry.path))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Sync password required or incorrect".to_string());
    }

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {} (HTTP {})",
            job.file_entry.path,
            response.status()
        ));
    }

    if let Some(parent) = job.dest.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

    let temp_path = part_path(&job.dest);
    let mut file = File::create(&temp_path)
        .await
        .map_err(|e| format!("Failed to create {}: {e}", temp_path.display()))?;

    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Failed to read {}: {e}", job.file_entry.path))?
    {
        if cancel.load(Ordering::Relaxed) {
            let _ = fs::remove_file(&temp_path).await;
            return Err("Sync cancelled".to_string());
        }

        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write {}: {e}", temp_path.display()))?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush {}: {e}", temp_path.display()))?;
    drop(file);

    if downloaded != job.file_entry.size {
        let _ = fs::remove_file(&temp_path).await;
        return Err(format!(
            "Incomplete download for {} (expected {} bytes, got {})",
            job.file_entry.path, job.file_entry.size, downloaded
        ));
    }

    let checksum = format!("sha256:{}", hex_encode(hasher.finalize()));
    if checksum != job.file_entry.checksum {
        let _ = fs::remove_file(&temp_path).await;
        return Err(format!(
            "Checksum mismatch for {} (expected {}, got {})",
            job.file_entry.path, job.file_entry.checksum, checksum
        ));
    }

    fs::rename(&temp_path, &job.dest)
        .await
        .map_err(|e| format!("Failed to finalize {}: {e}", job.dest.to_string_lossy()))?;

    state.mark_verified(&job.package_id, &job.file_entry, &checksum)?;

    Ok(downloaded)
}

pub async fn download_manifest_files(
    app: AppHandle,
    instance_id: &str,
    host: &str,
    port: u16,
    sync_password: Option<&str>,
    data_dir: &Path,
    manifest: &NerevarManifest,
    force: bool,
    cancel: Arc<AtomicBool>,
) -> Result<DownloadOutcome, String> {
    if force {
        clear_sync_state(data_dir)?;
    }

    let state = SharedSyncState::load(data_dir, manifest)?;

    if !force {
        let verified = count_verified_bytes(manifest, &state);
        let bytes_total = manifest.total_download_bytes.max(1);
        emit_sync_progress(
            &app,
            instance_id,
            SyncPhase::VerifyingExisting,
            "Verifying files already on disk (resume)",
            verified,
            bytes_total,
            0,
            0,
            None,
        );

        let data_dir_owned = data_dir.to_path_buf();
        let manifest_owned = manifest.clone();
        let state_for_adopt = state.clone();
        tauri::async_runtime::spawn_blocking(move || {
            adopt_existing_files_into_state(
                &data_dir_owned,
                &manifest_owned,
                &state_for_adopt,
                package_abs_path,
            )
        })
        .await
        .map_err(|e| format!("Verify task failed: {e}"))??;
    }

    let jobs = collect_download_jobs(data_dir, manifest, host, port, force, &state)?;
    let skipped_bytes = count_verified_bytes(manifest, &state);
    let bytes_total = manifest.total_download_bytes.max(1);
    let bytes_done = Arc::new(AtomicU64::new(skipped_bytes));
    let files_done = Arc::new(AtomicU64::new(0));
    let manifest_file_count: u64 = manifest
        .packages
        .iter()
        .map(|package| package.files.len() as u64)
        .sum();
    let files_remaining = jobs.len() as u64;
    let files_already_verified = manifest_file_count.saturating_sub(files_remaining);
    emit_sync_progress(
        &app,
        instance_id,
        SyncPhase::Downloading,
        if jobs.is_empty() {
            "All files already verified".to_string()
        } else if files_already_verified > 0 {
            format!(
                "Resuming download — {} files remaining ({} concurrent)",
                jobs.len(),
                MAX_CONCURRENT_DOWNLOADS
            )
        } else {
            format!(
                "Downloading {} files ({} concurrent)",
                jobs.len(),
                MAX_CONCURRENT_DOWNLOADS
            )
        },
        bytes_done.load(Ordering::Relaxed),
        bytes_total,
        files_already_verified,
        manifest_file_count,
        None,
    );

    if jobs.is_empty() {
        state.flush()?;
        return Ok(DownloadOutcome::Complete {
            bytes_done: bytes_done.load(Ordering::Relaxed),
        });
    }

    let client = sync_download_client()?;
    let queue = Arc::new(JobQueue::new(jobs));
    let sync_password = sync_password.map(str::to_string);
    let worker_count = MAX_CONCURRENT_DOWNLOADS.min(queue.jobs.len());
    let progress_throttler = Arc::new(ProgressThrottler::new());
    let mut workers = JoinSet::new();

    for _ in 0..worker_count {
        spawn_download_worker(
            &mut workers,
            app.clone(),
            instance_id.to_string(),
            client.clone(),
            sync_password.clone(),
            queue.clone(),
            state.clone(),
            cancel.clone(),
            bytes_done.clone(),
            files_done.clone(),
            manifest_file_count,
            bytes_total,
            progress_throttler.clone(),
        );
    }

    let mut first_error: Option<String> = None;

    while let Some(result) = workers.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) if err == "Sync cancelled" => {
                workers.abort_all();
                let _ = state.flush();
                return Ok(DownloadOutcome::Cancelled {
                    bytes_done: bytes_done.load(Ordering::Relaxed),
                    bytes_total,
                });
            }
            Ok(Err(err)) => {
                if first_error.is_none() {
                    cancel.store(true, Ordering::Relaxed);
                    first_error = Some(err);
                }
                workers.abort_all();
            }
            Err(err) => {
                if first_error.is_none() {
                    cancel.store(true, Ordering::Relaxed);
                    first_error = Some(format!("Download worker failed: {err}"));
                }
                workers.abort_all();
            }
        }
    }

    let _ = state.flush();

    if let Some(err) = first_error {
        return Err(err);
    }

    if cancel.load(Ordering::Relaxed) {
        return Ok(DownloadOutcome::Cancelled {
            bytes_done: bytes_done.load(Ordering::Relaxed),
            bytes_total,
        });
    }

    emit_sync_progress(
        &app,
        instance_id,
        SyncPhase::Downloading,
        "Download complete",
        bytes_done.load(Ordering::Relaxed),
        bytes_total,
        manifest_file_count,
        manifest_file_count,
        None,
    );

    Ok(DownloadOutcome::Complete {
        bytes_done: bytes_done.load(Ordering::Relaxed),
    })
}

#[allow(clippy::too_many_arguments)]
fn spawn_download_worker(
    workers: &mut JoinSet<Result<(), String>>,
    app: AppHandle,
    instance_id: String,
    client: Client,
    sync_password: Option<String>,
    queue: Arc<JobQueue>,
    state: Arc<SharedSyncState>,
    cancel: Arc<AtomicBool>,
    bytes_done: Arc<AtomicU64>,
    files_done: Arc<AtomicU64>,
    manifest_file_count: u64,
    bytes_total: u64,
    progress_throttler: Arc<ProgressThrottler>,
) {
    let files_remaining = queue.jobs.len() as u64;
    let files_already_verified = manifest_file_count.saturating_sub(files_remaining);

    workers.spawn(async move {
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Sync cancelled".to_string());
            }

            let Some(job) = queue.pop() else {
                return Ok(());
            };

            let password = sync_password.as_deref();
            let downloaded = download_one_file(&client, password, job, &state, &cancel).await?;
            bytes_done.fetch_add(downloaded, Ordering::Relaxed);
            let completed_this_run = files_done.fetch_add(1, Ordering::Relaxed) + 1;
            let done = bytes_done.load(Ordering::Relaxed);
            let files_done_total = files_already_verified + completed_this_run;
            let force = completed_this_run == files_remaining;

            maybe_emit_progress(
                &progress_throttler,
                &app,
                &instance_id,
                SyncPhase::Downloading,
                format!("Downloaded {files_done_total}/{manifest_file_count} files"),
                done,
                bytes_total,
                files_done_total,
                manifest_file_count,
                Some(job.file_entry.path.clone()),
                force,
            );
        }
    });
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::{ManifestPackage, PackageKind, ResolvedOpenMwConfig};
    use crate::instance_settings::InstanceSettings;
    use crate::sync_client::sync_state::load_sync_state;
    use std::fs;

    fn empty_manifest(packages: Vec<ManifestPackage>) -> NerevarManifest {
        NerevarManifest {
            version: 1,
            instance_id: "test".into(),
            instance_name: "test".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            base_game_data: None,
            packages,
            resolved: ResolvedOpenMwConfig {
                encoding: "win1252".into(),
                data_paths: vec![],
                content: vec![],
            },
            total_download_bytes: 0,
            tes3mp_server_port: 25565,
            tes3mp_server_password: String::new(),
            required_data_files: vec![],
            instance_settings: InstanceSettings::default(),
        }
    }

    fn sample_package(id: &str, relative_dir: &str, files: Vec<ManifestFileEntry>) -> ManifestPackage {
        let total_size_bytes = files.iter().map(|file| file.size).sum();
        ManifestPackage {
            id: id.to_string(),
            name: id.to_string(),
            kind: PackageKind::Mod,
            relative_dir: relative_dir.to_string(),
            priority: 0,
            tree_checksum: "sha256:tree".to_string(),
            total_size_bytes,
            file_count: files.len() as u32,
            files,
            plugins: vec![],
        }
    }

    #[test]
    fn skips_files_recorded_in_sync_state() {
        let data_dir = std::env::temp_dir().join(format!("nerevar-download-test-{}", uuid::Uuid::new_v4()));
        let nerevar_dir = data_dir.join(".nerevar");
        fs::create_dir_all(&nerevar_dir).unwrap();

        let content = b"verified";
        let checksum = {
            let path = data_dir.join("probe.bin");
            fs::write(&path, content).unwrap();
            let sum = hash_file_checksum(&path).unwrap();
            let _ = fs::remove_file(&path);
            sum
        };

        let entry = ManifestFileEntry {
            path: "done.txt".to_string(),
            size: content.len() as u64,
            checksum,
        };
        let manifest = empty_manifest(vec![sample_package("pkg-1", "mods/foo", vec![entry.clone()])]);

        let mut state_file = load_sync_state(&data_dir, &manifest).unwrap();
        state_file
            .completed
            .insert("pkg-1\x1fdone.txt".to_string(), entry.checksum.clone());
        crate::sync_client::sync_state::save_sync_state_file(&data_dir, &state_file).unwrap();

        let state = SharedSyncState::load(&data_dir, &manifest).unwrap();
        let jobs = collect_download_jobs(&data_dir, &manifest, "127.0.0.1", 8080, false, &state).unwrap();
        assert!(jobs.is_empty());

        let _ = fs::remove_dir_all(&data_dir);
    }
}
