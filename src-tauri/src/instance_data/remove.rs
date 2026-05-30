use std::path::Path;

use super::load_order::{load_load_order, save_load_order};
use super::manifest::build_manifest;
use super::paths::{manifest_path, package_abs_path};
use super::scan::should_skip_package_dir;
use super::types::LoadOrder;

pub fn delete_package(
    instance_id: &str,
    instance_name: &str,
    instance_root: &Path,
    data_dir: &Path,
    entry_id: &str,
) -> Result<LoadOrder, String> {
    let mut load_order = load_load_order(data_dir)?;
    let entry_index = load_order
        .entries
        .iter()
        .position(|entry| entry.id == entry_id)
        .ok_or_else(|| format!("Load order entry not found: {entry_id}"))?;
    let entry = load_order.entries[entry_index].clone();

    validate_relative_dir(&entry.relative_dir)?;
    if should_skip_package_dir(&entry.relative_dir) {
        return Err(format!(
            "Cannot delete protected directory: {}",
            entry.relative_dir
        ));
    }

    let package_path = package_abs_path(data_dir, &entry.relative_dir);
    ensure_path_within_data_dir(data_dir, &package_path)?;

    if package_path.exists() {
        std::fs::remove_dir_all(&package_path).map_err(|error| {
            format!(
                "Failed to delete {}: {error}",
                package_path.display()
            )
        })?;
    }

    load_order.entries.remove(entry_index);
    save_load_order(data_dir, &load_order)?;

    if manifest_path(data_dir).exists() {
        let mut no_progress = None;
        build_manifest(
            instance_id,
            instance_name,
            instance_root,
            data_dir,
            &load_order,
            &mut no_progress,
        )?;
    }

    Ok(load_order)
}

fn validate_relative_dir(relative_dir: &str) -> Result<(), String> {
    if relative_dir.trim().is_empty() {
        return Err("Package path is empty".into());
    }
    if relative_dir.contains("..")
        || relative_dir.contains('/')
        || relative_dir.contains('\\')
        || relative_dir.contains(':')
    {
        return Err(format!("Invalid package path: {relative_dir}"));
    }
    Ok(())
}

fn ensure_path_within_data_dir(data_dir: &Path, package_path: &Path) -> Result<(), String> {
    let data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("Failed to resolve data directory: {error}"))?;

    if !package_path.exists() {
        return Ok(());
    }

    let package_path = package_path
        .canonicalize()
        .map_err(|error| format!("Failed to resolve package directory: {error}"))?;

    if !package_path.starts_with(&data_dir) {
        return Err("Package path escapes instance data directory".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::paths::load_order_path;
    use crate::instance_data::types::{LoadOrderEntry, PackageKind, LOAD_ORDER_VERSION};

    fn write_load_order(data_dir: &Path, entries: Vec<LoadOrderEntry>) {
        std::fs::create_dir_all(data_dir.join(".nerevar")).unwrap();
        let load_order = LoadOrder {
            version: LOAD_ORDER_VERSION,
            base_game_data: None,
            content_order: None,
            entries,
        };
        std::fs::write(
            load_order_path(data_dir),
            serde_json::to_string_pretty(&load_order).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn delete_package_removes_directory_and_load_order_entry() {
        let root = std::env::temp_dir().join(format!("nerevar-delete-{}", std::process::id()));
        let data_dir = root.join("data");
        let package_dir = data_dir.join("Sample Mod");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&package_dir).unwrap();
        std::fs::write(package_dir.join("sample.esp"), b"plugin").unwrap();

        write_load_order(
            &data_dir,
            vec![LoadOrderEntry {
                id: "entry-1".into(),
                name: "Sample Mod".into(),
                kind: PackageKind::Mod,
                relative_dir: "Sample Mod".into(),
                enabled: true,
                priority: 10,
                plugins: vec![],
                tree_checksum: None,
            }],
        );

        let load_order = delete_package("inst-1", "Test", &root, &data_dir, "entry-1").unwrap();
        assert!(load_order.entries.is_empty());
        assert!(!package_dir.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(validate_relative_dir("../escape").is_err());
        assert!(validate_relative_dir("nested/mod").is_err());
    }
}
