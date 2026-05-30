use std::path::{Path, PathBuf};

use crate::data::InstanceConfig;
use crate::instance_setup::INSTANCE_TES3MP_DIR;

pub const NEREVAR_DIR: &str = ".nerevar";
pub const INSTANCE_DATA_DIR: &str = "data";
pub const LOAD_ORDER_FILE: &str = "load-order.json";
pub const MANIFEST_FILE: &str = "manifest.json";
pub const LAUNCH_CFG_DIR: &str = "launch";
pub const LAUNCH_CFG_FILE: &str = "openmw.launch.cfg";

pub fn nerevar_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(NEREVAR_DIR)
}

pub fn load_order_path(data_dir: &Path) -> PathBuf {
    nerevar_dir(data_dir).join(LOAD_ORDER_FILE)
}

pub fn manifest_path(data_dir: &Path) -> PathBuf {
    nerevar_dir(data_dir).join(MANIFEST_FILE)
}

pub fn launch_cfg_dir(data_dir: &Path) -> PathBuf {
    nerevar_dir(data_dir).join(LAUNCH_CFG_DIR)
}

pub fn launch_cfg_path(data_dir: &Path) -> PathBuf {
    launch_cfg_dir(data_dir).join(LAUNCH_CFG_FILE)
}

pub fn package_abs_path(data_dir: &Path, relative_dir: &str) -> PathBuf {
    data_dir.join(relative_dir)
}

/// Directory containing mod/replacer packages (immediate child folders of `data/`).
///
/// Config may incorrectly store the instance root as `data_dir`; when that layout
/// is detected (e.g. sibling `tes3mp/`), use `{instance_root}/data` instead.
pub fn resolve_package_data_dir(instance: &InstanceConfig) -> PathBuf {
    effective_data_dir(
        Path::new(&instance.path),
        Path::new(&instance.data_dir),
    )
}

fn effective_data_dir(instance_root: &Path, configured: &Path) -> PathBuf {
    let nested = instance_root.join(INSTANCE_DATA_DIR);

    // Misconfigured: scanning instance root shows `data/` + `tes3mp/` as packages.
    if configured.join(INSTANCE_TES3MP_DIR).is_dir() {
        return nested;
    }

    if configured == instance_root {
        return nested;
    }

    configured.to_path_buf()
}

/// Ensure `.nerevar/launch/` and `.nerevar/cache/` exist under the instance data directory.
pub fn ensure_instance_data_layout(data_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(launch_cfg_dir(data_dir)).map_err(|e| {
        format!(
            "Failed to create {}: {e}",
            launch_cfg_dir(data_dir).display()
        )
    })?;
    std::fs::create_dir_all(nerevar_dir(data_dir).join("cache")).map_err(|e| {
        format!(
            "Failed to create {}: {e}",
            nerevar_dir(data_dir).join("cache").display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::InstanceConfig;

    fn test_instance(path: &Path, data_dir: &Path) -> InstanceConfig {
        InstanceConfig {
            id: "test".into(),
            name: "Test".into(),
            description: String::new(),
            path: path.to_string_lossy().into_owned(),
            data_dir: data_dir.to_string_lossy().into_owned(),
            release_id: None,
            remote_host: None,
            remote_sync_port: None,
            last_synced_at: None,
            tes3mp_server_port: None,
        }
    }

    #[test]
    fn resolves_nested_data_when_configured_dir_is_instance_root() {
        let root = std::env::temp_dir().join(format!("nerevar-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::create_dir_all(root.join("tes3mp")).unwrap();

        let instance = test_instance(&root, &root);
        assert_eq!(
            resolve_package_data_dir(&instance),
            root.join("data")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn keeps_explicit_data_dir_when_already_correct() {
        let root = std::env::temp_dir().join(format!("nerevar-paths2-{}", std::process::id()));
        let data = root.join("data");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(root.join("tes3mp")).unwrap();

        let instance = test_instance(&root, &data);
        assert_eq!(resolve_package_data_dir(&instance), data);

        let _ = std::fs::remove_dir_all(&root);
    }
}
