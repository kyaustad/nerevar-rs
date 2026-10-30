use std::path::Path;

use super::load_order::load_load_order;
use super::manifest::load_manifest;
use super::resolver::{resolve_load_order, resolve_synced_load_order};
use super::types::ResolvedOpenMwConfig;

/// The instance's data folders and content files, from the host's manifest when there is one.
pub fn resolve_instance_openmw_config(data_dir: &Path) -> Result<ResolvedOpenMwConfig, String> {
    let load_order = load_load_order(data_dir)?;
    if let Ok(manifest) = load_manifest(data_dir) {
        resolve_synced_load_order(data_dir, &load_order, &manifest)
    } else {
        resolve_load_order(data_dir, &load_order)
    }
}
