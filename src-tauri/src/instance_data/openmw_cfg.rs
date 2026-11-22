use std::path::Path;

use super::load_order::load_load_order;
use super::manifest::load_manifest;
use super::resolver::{resolve_load_order, resolve_synced_load_order};
use super::types::ResolvedOpenMwConfig;

/// The instance's data folders and content files: a synced instance's from its host's manifest, an owned one's from
/// its load order. Owned instances keep a manifest too (what they serve to clients), which is only a snapshot.
pub fn resolve_instance_openmw_config(data_dir: &Path) -> Result<ResolvedOpenMwConfig, String> {
    let load_order = load_load_order(data_dir)?;
    if is_owned_instance(data_dir) {
        return resolve_load_order(data_dir, &load_order);
    }
    if let Ok(manifest) = load_manifest(data_dir) {
        resolve_synced_load_order(data_dir, &load_order, &manifest)
    } else {
        resolve_load_order(data_dir, &load_order)
    }
}

/// Whether `data_dir` belongs to one of the player's own instances, from Nerevar's config.
fn is_owned_instance(data_dir: &Path) -> bool {
    let Ok(config) = crate::config::nerevar_config::nerevar_config_file_path()
        .and_then(|path| if path.is_file() { crate::config::nerevar_config::load_or_create_nerevar_config_at(&path) } else { Err(String::new()) })
    else {
        return false;
    };
    let same = |dir: &Path| match (std::fs::canonicalize(dir), std::fs::canonicalize(data_dir)) {
        (Ok(a), Ok(b)) => a == b,
        _ => dir == data_dir,
    };
    config
        .owned_instances
        .iter()
        .flatten()
        .filter(|instance| instance.remote_host.is_none())
        .any(|instance| same(&super::paths::resolve_package_data_dir(instance)))
}
