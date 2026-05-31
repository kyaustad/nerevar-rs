use std::collections::HashSet;
use std::path::Path;

use super::paths::package_abs_path;
use super::scan::should_skip_package_dir;
use super::types::NerevarManifest;

/// Remove package directories and files on disk that are not listed in the manifest.
pub fn prune_local_against_manifest(
    data_dir: &Path,
    manifest: &NerevarManifest,
) -> Result<(), String> {
    prune_orphan_package_dirs(data_dir, manifest)?;
    prune_stale_package_files(data_dir, manifest)?;
    Ok(())
}

fn prune_orphan_package_dirs(data_dir: &Path, manifest: &NerevarManifest) -> Result<(), String> {
    if !data_dir.is_dir() {
        return Ok(());
    }

    let manifest_dirs: HashSet<&str> = manifest
        .packages
        .iter()
        .map(|package| package.relative_dir.as_str())
        .collect();

    for entry in std::fs::read_dir(data_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_package_dir(&name) {
            continue;
        }

        if !manifest_dirs.contains(name.as_str()) {
            std::fs::remove_dir_all(&path).map_err(|e| {
                format!("Failed to remove orphaned package {}: {e}", path.display())
            })?;
        }
    }

    Ok(())
}

fn prune_stale_package_files(data_dir: &Path, manifest: &NerevarManifest) -> Result<(), String> {
    for package in &manifest.packages {
        let package_dir = package_abs_path(data_dir, &package.relative_dir);
        if !package_dir.is_dir() {
            continue;
        }

        let allowed: HashSet<String> = package
            .files
            .iter()
            .map(|file| normalize_rel_path(&file.path))
            .collect();

        prune_files_recursive(&package_dir, &package_dir, &allowed)?;
    }

    Ok(())
}

fn prune_files_recursive(
    current: &Path,
    package_root: &Path,
    allowed: &HashSet<String>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            prune_files_recursive(&path, package_root, allowed)?;
            if is_empty_dir(&path)? {
                let _ = std::fs::remove_dir(&path);
            }
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let rel = path
            .strip_prefix(package_root)
            .map_err(|_| format!("Failed to resolve path under package root: {}", path.display()))?;
        let rel_str = normalize_rel_path(&rel.to_string_lossy());

        if !allowed.contains(&rel_str) {
            std::fs::remove_file(&path).map_err(|e| {
                format!("Failed to remove stale file {}: {e}", path.display())
            })?;
        }
    }

    Ok(())
}

fn is_empty_dir(path: &Path) -> Result<bool, String> {
    Ok(std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .next()
        .is_none())
}

fn normalize_rel_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::paths::ensure_instance_data_layout;
    use crate::instance_data::types::{
        ManifestFileEntry, ManifestPackage, NerevarManifest, PackageKind, ResolvedOpenMwConfig,
        MANIFEST_VERSION,
    };
    use crate::instance_settings::InstanceSettings;

    #[test]
    fn prune_removes_orphan_packages_and_stale_files() {
        let dir = std::env::temp_dir().join(format!("nerevar-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ensure_instance_data_layout(&dir).unwrap();

        let kept = dir.join("Kept Mod");
        let orphan = dir.join("Old Mod");
        std::fs::create_dir_all(&kept).unwrap();
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(kept.join("keep.txt"), b"keep").unwrap();
        std::fs::write(kept.join("stale.txt"), b"stale").unwrap();
        std::fs::write(orphan.join("gone.txt"), b"gone").unwrap();

        let manifest = NerevarManifest {
            version: MANIFEST_VERSION,
            instance_id: "test".into(),
            instance_name: "Test".into(),
            generated_at: "now".into(),
            base_game_data: None,
            packages: vec![ManifestPackage {
                id: "pkg-1".into(),
                name: "Kept Mod".into(),
                kind: PackageKind::Replacer,
                relative_dir: "Kept Mod".into(),
                priority: 10,
                tree_checksum: "abc".into(),
                total_size_bytes: 4,
                file_count: 1,
                files: vec![ManifestFileEntry {
                    path: "keep.txt".into(),
                    size: 4,
                    checksum: "deadbeef".into(),
                }],
                plugins: vec![],
            }],
            resolved: ResolvedOpenMwConfig {
                encoding: "win1252".into(),
                data_paths: vec![],
                content: vec![],
            },
            total_download_bytes: 4,
            tes3mp_server_port: 25565,
            tes3mp_server_password: String::new(),
            required_data_files: vec![],
            instance_settings: InstanceSettings::default(),
        };

        prune_local_against_manifest(&dir, &manifest).unwrap();

        assert!(kept.is_dir());
        assert!(orphan.is_dir() == false);
        assert!(kept.join("keep.txt").is_file());
        assert!(!kept.join("stale.txt").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
