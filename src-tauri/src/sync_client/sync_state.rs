use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::instance_data::{manifest_path, ManifestFileEntry, NerevarManifest};
use crate::sync_client::types::InstanceSyncStatus;

const SYNC_STATE_FILE: &str = "sync-state.json";
const SAVE_EVERY_N_FILES: u64 = 25;

fn sync_state_path(data_dir: &Path) -> PathBuf {
    manifest_path(data_dir)
        .parent()
        .map(|dir| dir.join(SYNC_STATE_FILE))
        .unwrap_or_else(|| data_dir.join(SYNC_STATE_FILE))
}

fn file_key(package_id: &str, relative_path: &str) -> String {
    format!("{package_id}\x1f{relative_path}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncStateFile {
    pub manifest_generated_at: String,
    pub completed: HashMap<String, String>,
}

impl SyncStateFile {
    fn new(manifest: &NerevarManifest) -> Self {
        Self {
            manifest_generated_at: manifest.generated_at.clone(),
            completed: HashMap::new(),
        }
    }

    fn matches_manifest(&self, manifest: &NerevarManifest) -> bool {
        self.manifest_generated_at == manifest.generated_at
    }

    fn is_verified(&self, package_id: &str, entry: &ManifestFileEntry) -> bool {
        self.completed
            .get(&file_key(package_id, &entry.path))
            .is_some_and(|checksum| checksum == &entry.checksum)
    }
}

pub fn load_sync_state(data_dir: &Path, manifest: &NerevarManifest) -> Result<SyncStateFile, String> {
    let path = sync_state_path(data_dir);
    if !path.is_file() {
        return Ok(SyncStateFile::new(manifest));
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read sync state: {e}"))?;
    let mut state: SyncStateFile =
        serde_json::from_str(&contents).map_err(|e| format!("Invalid sync-state.json: {e}"))?;

    if !state.matches_manifest(manifest) {
        state = SyncStateFile::new(manifest);
        save_sync_state_file(data_dir, &state)?;
    }

    Ok(state)
}

pub(crate) fn save_sync_state_file(data_dir: &Path, state: &SyncStateFile) -> Result<(), String> {
    let path = sync_state_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write sync state: {e}"))?;
    Ok(())
}

pub fn clear_sync_state(data_dir: &Path) -> Result<(), String> {
    let path = sync_state_path(data_dir);
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to clear sync state: {e}"))?;
    }
    Ok(())
}

pub fn sync_is_complete(state: &SyncStateFile, manifest: &NerevarManifest) -> bool {
    if !state.matches_manifest(manifest) {
        return false;
    }

    for package in &manifest.packages {
        for file in &package.files {
            if !state.is_verified(&package.id, file) {
                return false;
            }
        }
    }

    true
}

pub struct SharedSyncState {
    data_dir: PathBuf,
    inner: Mutex<SyncStateFile>,
    dirty: Mutex<bool>,
    saves_since_flush: Mutex<u64>,
}

impl SharedSyncState {
    pub fn load(data_dir: &Path, manifest: &NerevarManifest) -> Result<Arc<Self>, String> {
        let state = load_sync_state(data_dir, manifest)?;
        Ok(Arc::new(Self {
            data_dir: data_dir.to_path_buf(),
            inner: Mutex::new(state),
            dirty: Mutex::new(false),
            saves_since_flush: Mutex::new(0),
        }))
    }

    pub fn is_verified(&self, package_id: &str, entry: &ManifestFileEntry) -> bool {
        let Ok(guard) = self.inner.lock() else {
            return false;
        };
        guard.is_verified(package_id, entry)
    }

    pub fn mark_verified(
        &self,
        package_id: &str,
        entry: &ManifestFileEntry,
        checksum: &str,
    ) -> Result<(), String> {
        {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| "Sync state lock poisoned".to_string())?;
            guard
                .completed
                .insert(file_key(package_id, &entry.path), checksum.to_string());
        }

        if let Ok(mut dirty) = self.dirty.lock() {
            *dirty = true;
        }

        let mut saves = self
            .saves_since_flush
            .lock()
            .map_err(|_| "Sync state lock poisoned".to_string())?;
        *saves += 1;
        if *saves >= SAVE_EVERY_N_FILES {
            *saves = 0;
            drop(saves);
            self.flush()?;
        }

        Ok(())
    }

    pub fn flush(&self) -> Result<(), String> {
        let should_save = self.dirty.lock().map(|mut dirty| {
            if *dirty {
                *dirty = false;
                true
            } else {
                false
            }
        });

        if should_save.ok() == Some(true) {
            let guard = self
                .inner
                .lock()
                .map_err(|_| "Sync state lock poisoned".to_string())?;
            save_sync_state_file(&self.data_dir, &guard)?;
        }

        Ok(())
    }
}

pub fn hash_file_checksum(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("sha256:{}", hex_encode(hasher.finalize())))
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Files on disk from an older sync (size-only resume) that are not yet in sync-state.
pub fn adopt_existing_files_into_state(
    data_dir: &Path,
    manifest: &NerevarManifest,
    state: &SharedSyncState,
    package_abs: fn(&Path, &str) -> PathBuf,
) -> Result<u64, String> {
    let mut adopted = 0u64;

    for package in &manifest.packages {
        let package_dir = package_abs(data_dir, &package.relative_dir);
        for entry in &package.files {
            if state.is_verified(&package.id, entry) {
                adopted = adopted.saturating_add(entry.size);
                continue;
            }

            let dest = package_dir.join(&entry.path);
            let Ok(metadata) = std::fs::metadata(&dest) else {
                continue;
            };
            if !metadata.is_file() || metadata.len() != entry.size {
                continue;
            }

            let checksum = hash_file_checksum(&dest)?;
            if checksum == entry.checksum {
                state.mark_verified(&package.id, entry, &checksum)?;
                adopted = adopted.saturating_add(entry.size);
            }
        }
    }

    state.flush()?;
    Ok(adopted)
}

pub fn instance_sync_status_absent() -> InstanceSyncStatus {
    InstanceSyncStatus {
        has_manifest: false,
        can_resume: false,
        is_complete: false,
        bytes_verified: 0,
        bytes_total: 0,
        files_verified: 0,
        files_total: 0,
        percent_complete: 0,
    }
}

pub fn instance_sync_status(data_dir: &Path, manifest: &NerevarManifest) -> Result<InstanceSyncStatus, String> {
    let state = load_sync_state(data_dir, manifest)?;
    let mut files_verified = 0u32;
    let mut bytes_verified = 0u64;
    let mut files_total = 0u32;
    let mut bytes_total = 0u64;

    for package in &manifest.packages {
        for file in &package.files {
            files_total += 1;
            bytes_total = bytes_total.saturating_add(file.size);
            if state.is_verified(&package.id, file) {
                files_verified += 1;
                bytes_verified = bytes_verified.saturating_add(file.size);
            }
        }
    }

    let is_complete = sync_is_complete(&state, manifest);
    let percent_complete = if bytes_total > 0 {
        ((bytes_verified.min(bytes_total) * 100) / bytes_total).min(100) as u8
    } else {
        0
    };
    let can_resume = !is_complete && bytes_verified > 0;

    Ok(InstanceSyncStatus {
        has_manifest: true,
        can_resume,
        is_complete,
        bytes_verified,
        bytes_total,
        files_verified,
        files_total,
        percent_complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_key_is_stable() {
        assert_eq!(file_key("pkg", "textures/a.dds"), "pkg\x1ftextures/a.dds");
    }
}
