use std::path::Path;

use super::paths::{ensure_instance_data_layout, load_order_path};
use super::progress::{BackgroundOperationPhase, ProgressEmitter};
use super::scan::{new_entry_id, scan_data_directory};
use super::types::{LoadOrder, LoadOrderEntry, PluginEntry, LOAD_ORDER_VERSION};
use crate::openmw_ini_importer;

pub fn load_load_order(data_dir: &Path) -> Result<LoadOrder, String> {
    let path = load_order_path(data_dir);
    if !path.exists() {
        return Ok(LoadOrder::default());
    }
    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read load order: {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("Invalid load-order.json: {e}"))
}

pub fn save_load_order(data_dir: &Path, load_order: &LoadOrder) -> Result<(), String> {
    ensure_instance_data_layout(data_dir)?;
    let path = load_order_path(data_dir);
    let contents = serde_json::to_string_pretty(load_order)
        .map_err(|e| format!("Failed to serialize load order: {e}"))?;
    std::fs::write(&path, contents).map_err(|e| format!("Failed to write load order: {e}"))
}

/// Scan disk and merge results into load order (add new packages, update checksums/plugins).
pub fn scan_and_merge_load_order(
    data_dir: &Path,
    progress: &mut Option<ProgressEmitter>,
) -> Result<LoadOrder, String> {
    ensure_instance_data_layout(data_dir)?;
    let mut load_order = load_load_order(data_dir)?;
    if load_order.version == 0 {
        load_order.version = LOAD_ORDER_VERSION;
    }

    if load_order.base_game_data.is_none() {
        load_order.base_game_data = openmw_ini_importer::read_first_global_data_path();
    }

    let scanned = scan_data_directory(data_dir, progress)?;

    if let Some(emitter) = progress.as_mut() {
        emitter.emit(
            BackgroundOperationPhase::MergingLoadOrder,
            "Merging scan results into load order",
            0,
            1,
            None,
            true,
        );
    }

    merge_scanned_with_data_dir(data_dir, &mut load_order, scanned);

    if let Some(emitter) = progress.as_mut() {
        emitter.emit(
            BackgroundOperationPhase::SavingLoadOrder,
            "Saving load-order.json",
            0,
            1,
            None,
            true,
        );
    }

    save_load_order(data_dir, &load_order)?;
    Ok(load_order)
}

fn sync_plugins(entry: &mut LoadOrderEntry, scanned_plugins: &[String]) {
    let mut next = Vec::new();
    for file in scanned_plugins {
        let existing = entry
            .plugins
            .iter()
            .find(|p| p.file.eq_ignore_ascii_case(file));
        next.push(PluginEntry {
            file: file.clone(),
            enabled: existing.map(|p| p.enabled).unwrap_or(true),
        });
    }
    entry.plugins = next;
}

pub fn merge_scanned_with_data_dir(
    data_dir: &Path,
    load_order: &mut LoadOrder,
    scanned: Vec<super::types::ScannedPackage>, // ScannedPackage from scan module
) {
    for package in scanned {
        if let Some(existing) = load_order
            .entries
            .iter_mut()
            .find(|e| e.relative_dir == package.relative_dir)
        {
            existing.name = package.name;
            existing.kind = package.kind;
            if let Some(tree_checksum) = package.tree_checksum {
                existing.tree_checksum = Some(tree_checksum);
            }
            sync_plugins(existing, &package.plugins);
            continue;
        }

        let max_priority = load_order.entries.iter().map(|e| e.priority).max().unwrap_or(0);
        let plugins = package
            .plugins
            .iter()
            .map(|file| PluginEntry {
                file: file.clone(),
                enabled: true,
            })
            .collect();

        load_order.entries.push(LoadOrderEntry {
            id: new_entry_id(),
            name: package.name,
            kind: package.kind,
            relative_dir: package.relative_dir,
            enabled: true,
            priority: max_priority + 10,
            plugins,
            tree_checksum: package.tree_checksum,
        });
    }

    load_order.entries.retain(|entry| {
        super::paths::package_abs_path(data_dir, &entry.relative_dir).exists()
    });
}
