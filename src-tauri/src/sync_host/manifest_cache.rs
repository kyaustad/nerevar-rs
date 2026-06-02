use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::instance_data::{load_manifest, manifest_path};

#[derive(Clone)]
struct HostingPackageIndex {
    relative_dir: PathBuf,
    files: HashSet<String>,
}

#[derive(Clone)]
struct CachedHostingManifest {
    data_dir: PathBuf,
    manifest_mtime: SystemTime,
    packages: HashMap<String, HostingPackageIndex>,
}

#[derive(Default)]
pub struct HostingManifestCache {
    inner: Option<CachedHostingManifest>,
}

impl HostingManifestCache {
    pub fn clear(&mut self) {
        self.inner = None;
    }

    fn resolve(
        &self,
        data_dir: &Path,
        package_id: &str,
        relative_path: &str,
    ) -> Option<(PathBuf, PathBuf)> {
        let cache = self.inner.as_ref()?;
        if cache.data_dir != data_dir {
            return None;
        }

        let package = cache.packages.get(package_id)?;
        if !package.files.contains(relative_path) {
            return None;
        }

        Some((
            cache.data_dir.clone(),
            package.relative_dir.join(relative_path),
        ))
    }

    fn is_current(&self, data_dir: &Path, manifest_mtime: SystemTime) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|cache| cache.data_dir == data_dir && cache.manifest_mtime == manifest_mtime)
    }

    fn reload(&mut self, data_dir: &Path) -> Result<(), String> {
        let path = manifest_path(data_dir);
        let metadata = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to read manifest metadata: {e}"))?;
        let manifest_mtime = metadata
            .modified()
            .map_err(|e| format!("Failed to read manifest modified time: {e}"))?;

        if self.is_current(data_dir, manifest_mtime) {
            return Ok(());
        }

        let manifest = load_manifest(data_dir)?;
        let mut packages = HashMap::with_capacity(manifest.packages.len());

        for package in &manifest.packages {
            let files = package.files.iter().map(|file| file.path.clone()).collect();
            packages.insert(
                package.id.clone(),
                HostingPackageIndex {
                    relative_dir: PathBuf::from(&package.relative_dir),
                    files,
                },
            );
        }

        self.inner = Some(CachedHostingManifest {
            data_dir: data_dir.to_path_buf(),
            manifest_mtime,
            packages,
        });

        Ok(())
    }
}

pub type SharedHostingManifestCache = Arc<RwLock<HostingManifestCache>>;

pub fn new_shared_hosting_manifest_cache() -> SharedHostingManifestCache {
    Arc::new(RwLock::new(HostingManifestCache::default()))
}

pub fn get_package_file_path(
    cache: &SharedHostingManifestCache,
    data_dir: &Path,
    package_id: &str,
    relative_path: &str,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    let path = manifest_path(data_dir);
    let manifest_mtime = std::fs::metadata(&path)
        .map_err(|e| format!("Failed to read manifest metadata: {e}"))?
        .modified()
        .map_err(|e| format!("Failed to read manifest modified time: {e}"))?;

    if let Ok(guard) = cache.read() {
        if guard.is_current(data_dir, manifest_mtime) {
            return Ok(guard.resolve(data_dir, package_id, relative_path));
        }
    }

    let mut guard = cache
        .write()
        .map_err(|_| "Manifest cache lock poisoned".to_string())?;
    if !guard.is_current(data_dir, manifest_mtime) {
        guard.reload(data_dir)?;
    }

    Ok(guard.resolve(data_dir, package_id, relative_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_reuses_loaded_manifest_until_mtime_changes() {
        let cache = new_shared_hosting_manifest_cache();
        let data_dir = std::env::temp_dir().join(format!("nerevar-manifest-cache-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&data_dir).unwrap();

        let manifest_path = manifest_path(&data_dir);
        std::fs::write(
            &manifest_path,
            r#"{
                "version": 1,
                "instanceId": "test",
                "instanceName": "test",
                "generatedAt": "now",
                "packages": [{
                    "id": "pkg-1",
                    "name": "pkg-1",
                    "kind": "mod",
                    "relativeDir": "mods/foo",
                    "priority": 0,
                    "treeChecksum": "sha256:tree",
                    "totalSizeBytes": 1,
                    "fileCount": 1,
                    "files": [{ "path": "a.txt", "size": 1, "checksum": "sha256:a" }],
                    "plugins": []
                }],
                "resolved": { "encoding": "win1252", "dataPaths": [], "content": [] },
                "totalDownloadBytes": 1,
                "tes3mpServerPort": 25565,
                "tes3mpServerPassword": "",
                "requiredDataFiles": [],
                "instanceSettings": {}
            }"#,
        )
        .unwrap();

        let resolved = get_package_file_path(&cache, &data_dir, "pkg-1", "a.txt").unwrap();
        assert!(resolved.is_some());

        let read_guard = cache.read().unwrap();
        assert!(read_guard.inner.is_some());
        drop(read_guard);

        let resolved_again = get_package_file_path(&cache, &data_dir, "pkg-1", "a.txt").unwrap();
        assert_eq!(resolved, resolved_again);

        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
