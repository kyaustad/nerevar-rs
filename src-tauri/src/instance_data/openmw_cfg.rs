use std::fs::File;
use std::path::Path;

use crate::openmw_ini_importer::MultiStrMap;

use super::load_order::load_load_order;
use super::manifest::load_manifest;
use super::paths::{ensure_instance_data_layout, launch_cfg_dir, launch_cfg_path};
use super::resolver::{resolve_load_order, resolve_synced_load_order};
use super::types::ResolvedOpenMwConfig;

pub fn resolve_instance_openmw_config(data_dir: &Path) -> Result<ResolvedOpenMwConfig, String> {
    let load_order = load_load_order(data_dir)?;
    if let Ok(manifest) = load_manifest(data_dir) {
        resolve_synced_load_order(data_dir, &load_order, &manifest)
    } else {
        resolve_load_order(data_dir, &load_order)
    }
}

/// Write `{data_dir}/.nerevar/launch/openmw.launch.cfg`.
pub fn write_instance_launch_cfg(
    data_dir: &Path,
    resolved: &ResolvedOpenMwConfig,
) -> Result<(), String> {
    ensure_instance_data_layout(data_dir)?;
    write_resolved_to_path(&launch_cfg_path(data_dir), resolved)
}

pub fn write_ephemeral_openmw_cfg(
    data_dir: &Path,
    resolved: &ResolvedOpenMwConfig,
) -> Result<String, String> {
    write_instance_launch_cfg(data_dir, resolved)?;
    Ok(launch_cfg_dir(data_dir).to_string_lossy().into_owned())
}

fn write_resolved_to_path(path: &Path, resolved: &ResolvedOpenMwConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create launch cfg directory: {e}"))?;
    }

    let mut cfg: MultiStrMap = std::collections::BTreeMap::new();
    cfg.insert("encoding".to_string(), vec![resolved.encoding.clone()]);
    cfg.insert("data".to_string(), resolved.data_paths.clone());
    cfg.insert("content".to_string(), resolved.content.clone());

    let mut file =
        File::create(path).map_err(|e| format!("Failed to create launch cfg: {e}"))?;
    crate::openmw_ini_importer::write_to_file(&mut file, &cfg)
        .map_err(|e| format!("Failed to write launch cfg: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::paths::{launch_cfg_dir, launch_cfg_path};
    use crate::instance_data::types::ResolvedOpenMwConfig;

    fn sample_resolved() -> ResolvedOpenMwConfig {
        ResolvedOpenMwConfig {
            encoding: "win1252".into(),
            data_paths: vec![
                "\"C:\\\\Morrowind\\\\Data Files\"".into(),
                "\"C:\\\\mods\\\\Better Bodies\"".into(),
            ],
            content: vec![
                "Morrowind.esm".into(),
                "Tribunal.esm".into(),
                "Bloodmoon.esm".into(),
                "Better Bodies.esp".into(),
            ],
        }
    }

    #[test]
    fn writes_openmw_launch_cfg_in_launch_directory() {
        let dir = std::env::temp_dir().join(format!("nerevar-openmw-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let returned = write_ephemeral_openmw_cfg(&dir, &sample_resolved()).unwrap();
        assert_eq!(returned, launch_cfg_dir(&dir).to_string_lossy());
        assert!(launch_cfg_path(&dir).is_file());

        let contents = std::fs::read_to_string(launch_cfg_path(&dir)).unwrap();
        assert!(contents.contains("content=Better Bodies.esp"));
        assert!(contents.contains("data="));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
