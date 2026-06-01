use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use reqwest::Client;
use tauri::{AppHandle, Emitter};
use tokio::task::JoinSet;

use crate::instance_data::{package_abs_path, ManifestFileEntry, NerevarManifest};
use crate::sync_auth::SYNC_PASSWORD_HEADER;

use super::types::{SyncPhase, SyncProgressEvent};

const MAX_CONCURRENT_DOWNLOADS: usize = 5;

fn user_agent() -> String {
    format!("Nerevar-{}", env!("CARGO_PKG_VERSION"))
}

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

struct DownloadJob {
    relative_path: String,
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

    fn len(&self) -> usize {
        self.jobs.len()
    }
}

fn file_needs_download(
    previous_manifest: Option<&NerevarManifest>,
    package_id: &str,
    entry: &ManifestFileEntry,
    dest: &Path,
) -> bool {
    if !dest.exists() {
        return true;
    }

    let Ok(metadata) = std::fs::metadata(dest) else {
        return true;
    };
    if metadata.len() != entry.size {
        return true;
    }

    let Some(previous) = previous_manifest else {
        return true;
    };
    let Some(package) = previous.packages.iter().find(|pkg| pkg.id == package_id) else {
        return true;
    };
    let Some(previous_file) = package.files.iter().find(|file| file.path == entry.path) else {
        return true;
    };

    previous_file.checksum != entry.checksum
}

fn collect_download_jobs(
    data_dir: &Path,
    manifest: &NerevarManifest,
    host: &str,
    port: u16,
    previous_manifest: Option<&NerevarManifest>,
) -> Result<(Vec<DownloadJob>, u64), String> {
    let base = base_url(host, port);
    let mut jobs = Vec::new();
    let mut bytes_to_download = 0u64;

    for package in &manifest.packages {
        let package_dir = package_abs_path(data_dir, &package.relative_dir);
        std::fs::create_dir_all(&package_dir)
            .map_err(|e| format!("Failed to create {}: {e}", package_dir.display()))?;

        for file_entry in &package.files {
            let dest = package_dir.join(&file_entry.path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            if !file_needs_download(previous_manifest, &package.id, file_entry, &dest) {
                continue;
            }

            let url = format!(
                "{}/packages/{}/files/{}",
                base,
                urlencoding::encode(&package.id),
                encode_path_segments(&file_entry.path)
            );

            bytes_to_download = bytes_to_download.saturating_add(file_entry.size);
            jobs.push(DownloadJob {
                relative_path: file_entry.path.clone(),
                dest,
                url,
            });
        }
    }

    Ok((jobs, bytes_to_download))
}

async fn download_one_file(
    client: &Client,
    sync_password: Option<&str>,
    job: &DownloadJob,
) -> Result<u64, String> {
    let mut request = client.get(&job.url).header("User-Agent", user_agent());
    if let Some(password) = sync_password.filter(|value| !value.is_empty()) {
        request = request.header(SYNC_PASSWORD_HEADER, password);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to download {}: {e}", job.relative_path))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Sync password required or incorrect".to_string());
    }

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {} (HTTP {})",
            job.relative_path,
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read {}: {e}", job.relative_path))?;

    if let Some(parent) = job.dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::write(&job.dest, &bytes).map_err(|e| {
        format!(
            "Failed to write {}: {e}",
            job.dest.to_string_lossy()
        )
    })?;

    Ok(bytes.len() as u64)
}

pub async fn download_manifest_files(
    app: AppHandle,
    instance_id: &str,
    host: &str,
    port: u16,
    sync_password: Option<&str>,
    data_dir: &Path,
    manifest: &NerevarManifest,
    previous_manifest: Option<&NerevarManifest>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let (jobs, bytes_to_download) =
        collect_download_jobs(data_dir, manifest, host, port, previous_manifest)?;
    let skipped_bytes = manifest
        .total_download_bytes
        .saturating_sub(bytes_to_download);

    let bytes_total = manifest.total_download_bytes.max(1);
    let bytes_done = Arc::new(AtomicU64::new(skipped_bytes));
    let files_done = Arc::new(AtomicU64::new(0));
    let files_total = jobs.len() as u64;

    emit_progress(
        &app,
        instance_id,
        SyncPhase::Downloading,
        if jobs.is_empty() {
            "All files already present".to_string()
        } else {
            format!(
                "Downloading {} files ({} concurrent)",
                jobs.len(),
                MAX_CONCURRENT_DOWNLOADS
            )
        },
        bytes_done.load(Ordering::Relaxed),
        bytes_total,
        None,
    );

    if jobs.is_empty() {
        return Ok(());
    }

    let client = Client::new();
    let queue = Arc::new(JobQueue::new(jobs));
    let sync_password = sync_password.map(str::to_string);
    let worker_count = MAX_CONCURRENT_DOWNLOADS.min(queue.len());
    let mut workers = JoinSet::new();

    for _ in 0..worker_count {
        spawn_download_worker(
            &mut workers,
            app.clone(),
            instance_id.to_string(),
            client.clone(),
            sync_password.clone(),
            queue.clone(),
            cancel.clone(),
            bytes_done.clone(),
            files_done.clone(),
            files_total,
            bytes_total,
        );
    }

    while let Some(result) = workers.join_next().await {
        result.map_err(|e| format!("Download worker failed: {e}"))??;

        if cancel.load(Ordering::Relaxed) {
            workers.abort_all();
            return Err("Sync cancelled".to_string());
        }
    }

    emit_progress(
        &app,
        instance_id,
        SyncPhase::Downloading,
        "Download complete",
        bytes_done.load(Ordering::Relaxed),
        bytes_total,
        None,
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn spawn_download_worker(
    workers: &mut JoinSet<Result<(), String>>,
    app: AppHandle,
    instance_id: String,
    client: Client,
    sync_password: Option<String>,
    queue: Arc<JobQueue>,
    cancel: Arc<AtomicBool>,
    bytes_done: Arc<AtomicU64>,
    files_done: Arc<AtomicU64>,
    files_total: u64,
    bytes_total: u64,
) {
    workers.spawn(async move {
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Sync cancelled".to_string());
            }

            let Some(job) = queue.pop() else {
                return Ok(());
            };

            emit_progress(
                &app,
                &instance_id,
                SyncPhase::Downloading,
                format!("Downloading {}", job.relative_path),
                bytes_done.load(Ordering::Relaxed),
                bytes_total,
                Some(job.relative_path.clone()),
            );

            let password = sync_password.as_deref();
            let downloaded = download_one_file(&client, password, job).await?;
            bytes_done.fetch_add(downloaded, Ordering::Relaxed);
            let completed = files_done.fetch_add(1, Ordering::Relaxed) + 1;

            emit_progress(
                &app,
                &instance_id,
                SyncPhase::Downloading,
                format!("Downloaded {} ({completed}/{files_total})", job.relative_path),
                bytes_done.load(Ordering::Relaxed),
                bytes_total,
                Some(job.relative_path.clone()),
            );
        }
    });
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
    use std::fs;

    fn empty_manifest(packages: Vec<ManifestPackage>) -> NerevarManifest {
        NerevarManifest {
            version: 1,
            instance_id: "test".into(),
            instance_name: "test".into(),
            generated_at: String::new(),
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

    fn sample_file(path: &str, checksum: &str, size: u64) -> ManifestFileEntry {
        ManifestFileEntry {
            path: path.to_string(),
            size,
            checksum: checksum.to_string(),
        }
    }

    #[test]
    fn skips_unchanged_files_when_previous_manifest_matches() {
        let data_dir = std::env::temp_dir().join(format!("nerevar-download-test-{}", uuid::Uuid::new_v4()));
        let package_dir = data_dir.join("mods").join("foo");
        fs::create_dir_all(&package_dir).unwrap();
        let dest = package_dir.join("unchanged.txt");
        fs::write(&dest, b"same content").unwrap();

        let checksum = "sha256:abc";
        let entry = sample_file("unchanged.txt", checksum, dest.metadata().unwrap().len());
        let manifest = empty_manifest(vec![sample_package("pkg-1", "mods/foo", vec![entry])]);

        let (jobs, bytes) =
            collect_download_jobs(&data_dir, &manifest, "127.0.0.1", 8080, Some(&manifest)).unwrap();
        assert!(jobs.is_empty());
        assert_eq!(bytes, 0);

        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn queues_files_when_checksum_changed() {
        let data_dir = std::env::temp_dir().join(format!("nerevar-download-test-{}", uuid::Uuid::new_v4()));
        let package_dir = data_dir.join("mods").join("foo");
        fs::create_dir_all(&package_dir).unwrap();
        let dest = package_dir.join("changed.txt");
        fs::write(&dest, b"old content").unwrap();

        let previous_entry = sample_file("changed.txt", "sha256:old", dest.metadata().unwrap().len());
        let remote_entry = sample_file("changed.txt", "sha256:new", 12);
        let previous = empty_manifest(vec![sample_package(
            "pkg-1",
            "mods/foo",
            vec![previous_entry],
        )]);
        let remote = empty_manifest(vec![sample_package(
            "pkg-1",
            "mods/foo",
            vec![remote_entry.clone()],
        )]);

        let (jobs, bytes) =
            collect_download_jobs(&data_dir, &remote, "127.0.0.1", 8080, Some(&previous)).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(bytes, remote_entry.size);
        assert_eq!(jobs[0].relative_path, "changed.txt");

        let _ = fs::remove_dir_all(&data_dir);
    }
}
