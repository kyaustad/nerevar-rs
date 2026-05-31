use std::path::Path;

use crate::instance_data::{
    load_load_order, save_load_order, LoadOrder, LoadOrderEntry, NerevarManifest, LOAD_ORDER_VERSION,
};
use crate::openmw_ini_importer::read_first_global_data_path;

pub fn apply_manifest_to_load_order(
    data_dir: &Path,
    manifest: &NerevarManifest,
) -> Result<LoadOrder, String> {
    let mut packages: Vec<_> = manifest.packages.iter().collect();
    packages.sort_by_key(|p| p.priority);

    let existing_base = load_load_order(data_dir)
        .ok()
        .and_then(|lo| lo.base_game_data)
        .filter(|p| !p.trim().is_empty());
    let base_game_data = existing_base
        .or_else(read_first_global_data_path)
        .or(manifest.base_game_data.clone());

    let load_order = LoadOrder {
        version: LOAD_ORDER_VERSION,
        base_game_data,
        content_order: None,
        entries: packages
            .into_iter()
            .map(|pkg| LoadOrderEntry {
                id: pkg.id.clone(),
                name: pkg.name.clone(),
                kind: pkg.kind,
                relative_dir: pkg.relative_dir.clone(),
                enabled: true,
                priority: pkg.priority,
                plugins: pkg.plugins.clone(),
                tree_checksum: Some(pkg.tree_checksum.clone()),
            })
            .collect(),
    };

    save_load_order(data_dir, &load_order)?;
    Ok(load_order)
}
