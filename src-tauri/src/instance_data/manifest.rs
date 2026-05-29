use std::path::Path;

use chrono::Utc;

use super::checksum::{directory_tree_checksum, file_checksum};
use super::paths::{manifest_path, package_abs_path};
use super::resolver::resolve_load_order;
use super::checksum::collect_package_files;
use super::types::{
    LoadOrder, ManifestPackage, ManifestValidationIssue, ManifestValidationResult,
    NerevarManifest, MANIFEST_VERSION,
};

use crate::instance_setup::{
    build_required_data_files, instance_tes3mp_dir, read_tes3mp_server_settings,
    write_required_data_files,
};

pub fn build_manifest(
    instance_id: &str,
    instance_name: &str,
    instance_root: &Path,
    data_dir: &Path,
    load_order: &LoadOrder,
) -> Result<NerevarManifest, String> {
    let resolved = resolve_load_order(data_dir, load_order)?;
    let server_settings = read_tes3mp_server_settings(&instance_tes3mp_dir(instance_root))?;
    let required_data_files = build_required_data_files(&resolved)?;
    write_required_data_files(
        &instance_tes3mp_dir(instance_root),
        &required_data_files,
    )?;
    let mut packages = Vec::new();
    let mut total_download_bytes = 0u64;

    let mut enabled_entries: Vec<_> = load_order.entries.iter().filter(|e| e.enabled).collect();
    enabled_entries.sort_by_key(|e| e.priority);

    for entry in enabled_entries {
        let package_dir = package_abs_path(data_dir, &entry.relative_dir);
        if !package_dir.exists() {
            continue;
        }

        let tree_checksum = directory_tree_checksum(&package_dir)?;
        let file_entries = collect_package_files(&package_dir)?;
        let package_size: u64 = file_entries.iter().map(|f| f.size).sum();
        total_download_bytes += package_size;

        packages.push(ManifestPackage {
            id: entry.id.clone(),
            name: entry.name.clone(),
            kind: entry.kind,
            relative_dir: entry.relative_dir.clone(),
            priority: entry.priority,
            tree_checksum,
            total_size_bytes: package_size,
            file_count: file_entries.len() as u32,
            files: file_entries,
            plugins: entry.plugins.clone(),
        });
    }

    let manifest = NerevarManifest {
        version: MANIFEST_VERSION,
        instance_id: instance_id.to_string(),
        instance_name: instance_name.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        base_game_data: load_order.base_game_data.clone(),
        packages,
        resolved,
        total_download_bytes,
        tes3mp_server_port: server_settings.port,
        tes3mp_server_password: server_settings.password,
        required_data_files,
    };

    let path = manifest_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write manifest: {e}"))?;

    Ok(manifest)
}

pub fn load_manifest(data_dir: &Path) -> Result<NerevarManifest, String> {
    let path = manifest_path(data_dir);
    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read manifest: {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("Invalid manifest.json: {e}"))
}

pub fn validate_manifest_against_disk(
    data_dir: &Path,
    manifest: &NerevarManifest,
) -> ManifestValidationResult {
    let mut issues = Vec::new();

    for package in &manifest.packages {
        let package_dir = package_abs_path(data_dir, &package.relative_dir);
        if !package_dir.exists() {
            issues.push(ManifestValidationIssue {
                package_id: package.id.clone(),
                relative_dir: package.relative_dir.clone(),
                message: "Package directory is missing".to_string(),
            });
            continue;
        }

        let current_tree = match directory_tree_checksum(&package_dir) {
            Ok(hash) => hash,
            Err(err) => {
                issues.push(ManifestValidationIssue {
                    package_id: package.id.clone(),
                    relative_dir: package.relative_dir.clone(),
                    message: err,
                });
                continue;
            }
        };

        if current_tree != package.tree_checksum {
            issues.push(ManifestValidationIssue {
                package_id: package.id.clone(),
                relative_dir: package.relative_dir.clone(),
                message: format!(
                    "Tree checksum mismatch (expected {}, got {})",
                    package.tree_checksum, current_tree
                ),
            });
        }

        for file_entry in &package.files {
            let file_path = package_dir.join(&file_entry.path);
            if !file_path.is_file() {
                issues.push(ManifestValidationIssue {
                    package_id: package.id.clone(),
                    relative_dir: package.relative_dir.clone(),
                    message: format!("Missing file: {}", file_entry.path),
                });
                continue;
            }

            match file_checksum(&file_path) {
                Ok(hash) if hash == file_entry.checksum => {}
                Ok(hash) => {
                    issues.push(ManifestValidationIssue {
                        package_id: package.id.clone(),
                        relative_dir: package.relative_dir.clone(),
                        message: format!(
                            "Checksum mismatch for {} (expected {}, got {})",
                            file_entry.path, file_entry.checksum, hash
                        ),
                    });
                }
                Err(err) => {
                    issues.push(ManifestValidationIssue {
                        package_id: package.id.clone(),
                        relative_dir: package.relative_dir.clone(),
                        message: err,
                    });
                }
            }
        }
    }

    ManifestValidationResult {
        valid: issues.is_empty(),
        issues,
    }
}
