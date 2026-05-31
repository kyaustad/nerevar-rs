use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use rayon::prelude::*;

use super::checksum::{hash_package_directory, PackageHashResult};
use super::paths::{manifest_path, package_abs_path};
use super::progress::{BackgroundOperationPhase, ProgressEmitter};
use super::resolver::resolve_load_order;
use super::types::{
    LoadOrder, LoadOrderEntry, ManifestPackage, ManifestValidationIssue,
    ManifestValidationResult, NerevarManifest, MANIFEST_VERSION,
};

use crate::instance_setup::{
    build_required_data_files, instance_tes3mp_dir, read_tes3mp_server_settings,
    write_required_data_files,
};
use crate::instance_settings::{
    apply_instance_settings_to_disk, load_instance_settings, write_launch_settings_overlay,
};

struct PackageWorkItem {
    index: usize,
    entry: LoadOrderEntry,
}

pub fn build_manifest(
    instance_id: &str,
    instance_name: &str,
    instance_root: &Path,
    data_dir: &Path,
    load_order: &LoadOrder,
    progress: &mut Option<ProgressEmitter>,
) -> Result<NerevarManifest, String> {
    let shared_progress = progress.take().map(|emitter| Arc::new(Mutex::new(emitter)));

    if let Some(emitter) = shared_progress.as_ref() {
        if let Ok(mut guard) = emitter.lock() {
            guard.emit(
                BackgroundOperationPhase::ResolvingLoadOrder,
                "Resolving load order and plugin paths",
                0,
                1,
                None,
                true,
            );
        }
    }

    let resolved = resolve_load_order(data_dir, load_order)?;

    if let Some(emitter) = shared_progress.as_ref() {
        if let Ok(mut guard) = emitter.lock() {
            guard.emit(
                BackgroundOperationPhase::UpdatingServerMetadata,
                "Updating TES3MP required-data-files.json",
                0,
                1,
                None,
                true,
            );
        }
    }

    let server_settings = read_tes3mp_server_settings(&instance_tes3mp_dir(instance_root))?;
    let instance_settings = load_instance_settings(data_dir)?;
    apply_instance_settings_to_disk(instance_root, data_dir, &instance_settings)?;
    let required_data_files = build_required_data_files(&resolved)?;
    write_required_data_files(
        &instance_tes3mp_dir(instance_root),
        &required_data_files,
    )?;

    let mut enabled_entries: Vec<_> = load_order.entries.iter().filter(|e| e.enabled).collect();
    enabled_entries.sort_by_key(|e| e.priority);

    let work: Vec<PackageWorkItem> = enabled_entries
        .into_iter()
        .enumerate()
        .filter(|(_, entry)| package_abs_path(data_dir, &entry.relative_dir).exists())
        .map(|(index, entry)| PackageWorkItem {
            index,
            entry: entry.clone(),
        })
        .collect();

    let package_total = work.len() as u64;
    let data_dir = data_dir.to_path_buf();

    let hashed_packages: Result<Vec<(usize, ManifestPackage)>, String> = work
        .par_iter()
        .map(|item| {
            let step = item.index as u64 + 1;
            if let Some(emitter) = shared_progress.as_ref() {
                if let Ok(mut guard) = emitter.lock() {
                    guard.emit(
                        BackgroundOperationPhase::HashingPackage,
                        format!(
                            "Processing {} ({step}/{package_total})",
                            item.entry.name
                        ),
                        step,
                        package_total,
                        Some(item.entry.relative_dir.clone()),
                        true,
                    );
                }
            }

            let package_dir = package_abs_path(&data_dir, &item.entry.relative_dir);
            let PackageHashResult {
                tree_checksum,
                files,
                total_size_bytes,
            } = hash_package_directory(&package_dir, shared_progress.clone())?;

            Ok((
                item.index,
                ManifestPackage {
                    id: item.entry.id.clone(),
                    name: item.entry.name.clone(),
                    kind: item.entry.kind,
                    relative_dir: item.entry.relative_dir.clone(),
                    priority: item.entry.priority,
                    tree_checksum,
                    total_size_bytes,
                    file_count: files.len() as u32,
                    files,
                    plugins: item.entry.plugins.clone(),
                },
            ))
        })
        .collect();

    let mut hashed_packages = hashed_packages?;
    hashed_packages.sort_by_key(|(index, _)| *index);

    let packages: Vec<ManifestPackage> = hashed_packages
        .into_iter()
        .map(|(_, package)| package)
        .collect();
    let total_download_bytes = packages.iter().map(|package| package.total_size_bytes).sum();

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
        instance_settings,
    };

    if let Some(emitter) = shared_progress.as_ref() {
        if let Ok(mut guard) = emitter.lock() {
            guard.emit(
                BackgroundOperationPhase::WritingManifest,
                "Writing manifest.json",
                1,
                1,
                None,
                true,
            );
        }
    }

    let path = manifest_path(data_dir.as_path());
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
    let data_dir = data_dir.to_path_buf();
    let issues: Vec<ManifestValidationIssue> = manifest
        .packages
        .par_iter()
        .flat_map(|package| validate_package_on_disk(&data_dir, package))
        .collect();

    ManifestValidationResult {
        valid: issues.is_empty(),
        issues,
    }
}

fn validate_package_on_disk(
    data_dir: &Path,
    package: &super::types::ManifestPackage,
) -> Vec<ManifestValidationIssue> {
    let mut issues = Vec::new();
    let package_dir = package_abs_path(data_dir, &package.relative_dir);
    if !package_dir.exists() {
        issues.push(ManifestValidationIssue {
            package_id: package.id.clone(),
            relative_dir: package.relative_dir.clone(),
            message: "Package directory is missing".to_string(),
        });
        return issues;
    }

    let hashed = match hash_package_directory(&package_dir, None) {
        Ok(result) => result,
        Err(err) => {
            issues.push(ManifestValidationIssue {
                package_id: package.id.clone(),
                relative_dir: package.relative_dir.clone(),
                message: err,
            });
            return issues;
        }
    };

    if hashed.tree_checksum != package.tree_checksum {
        issues.push(ManifestValidationIssue {
            package_id: package.id.clone(),
            relative_dir: package.relative_dir.clone(),
            message: format!(
                "Tree checksum mismatch (expected {}, got {})",
                package.tree_checksum, hashed.tree_checksum
            ),
        });
    }

    for expected in &package.files {
        match hashed.files.iter().find(|file| file.path == expected.path) {
            None => issues.push(ManifestValidationIssue {
                package_id: package.id.clone(),
                relative_dir: package.relative_dir.clone(),
                message: format!("Missing file: {}", expected.path),
            }),
            Some(actual) if actual.checksum != expected.checksum => {
                issues.push(ManifestValidationIssue {
                    package_id: package.id.clone(),
                    relative_dir: package.relative_dir.clone(),
                    message: format!(
                        "Checksum mismatch for {} (expected {}, got {})",
                        expected.path, expected.checksum, actual.checksum
                    ),
                });
            }
            Some(actual) if actual.size != expected.size => {
                issues.push(ManifestValidationIssue {
                    package_id: package.id.clone(),
                    relative_dir: package.relative_dir.clone(),
                    message: format!(
                        "Size mismatch for {} (expected {}, got {})",
                        expected.path, expected.size, actual.size
                    ),
                });
            }
            _ => {}
        }
    }

    issues
}
