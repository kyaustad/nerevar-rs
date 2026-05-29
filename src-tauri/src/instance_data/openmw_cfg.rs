use std::fs::File;
use std::path::{Path, PathBuf};

use crate::openmw_ini_importer::MultiStrMap;

use super::load_order::load_load_order;
use super::manifest::load_manifest;
use super::paths::{ensure_instance_data_layout, launch_cfg_dir, launch_cfg_path};
use super::resolver::{resolve_load_order, resolve_synced_load_order};
use super::types::ResolvedOpenMwConfig;

const NEREVAR_OVERLAY_HEADER: &str =
    "# Nerevar managed data paths and plugin load order (do not edit manually)\n";

pub fn resolve_instance_openmw_config(data_dir: &Path) -> Result<ResolvedOpenMwConfig, String> {
    let load_order = load_load_order(data_dir)?;
    if let Ok(manifest) = load_manifest(data_dir) {
        resolve_synced_load_order(data_dir, &load_order, &manifest)
    } else {
        resolve_load_order(data_dir, &load_order)
    }
}

/// Write `{data_dir}/.nerevar/launch/openmw.cfg` and return that directory path.
pub fn write_ephemeral_openmw_cfg(
    data_dir: &Path,
    resolved: &ResolvedOpenMwConfig,
) -> Result<String, String> {
    ensure_instance_data_layout(data_dir)?;
    let cfg_dir = launch_cfg_dir(data_dir);
    let path = launch_cfg_path(data_dir);
    write_resolved_to_path(&path, resolved)?;
    Ok(cfg_dir.to_string_lossy().into_owned())
}

/// Install instance load order into `{tes3mp_dir}/openmw.cfg`, which TES3MP reads at startup.
///
/// TES3MP loads the user's global OpenMW config and then `./openmw.cfg` next to `tes3mp.exe`.
/// The local file must contain the full plugin list. When the global config also lists
/// `content=` entries, callers should temporarily disable those lines during launch.
pub fn write_tes3mp_launch_openmw_cfg(
    tes3mp_dir: &Path,
    data_dir: &Path,
    resolved: &ResolvedOpenMwConfig,
) -> Result<PathBuf, String> {
    write_ephemeral_openmw_cfg(data_dir, resolved)?;

    let target = tes3mp_dir.join("openmw.cfg");
    let base = std::fs::read_to_string(&target).unwrap_or_default();
    let merged = merge_openmw_cfg_overlay(&base, resolved);
    std::fs::write(&target, merged).map_err(|e| {
        format!(
            "Failed to write TES3MP openmw.cfg at {}: {e}",
            target.display()
        )
    })?;
    Ok(target)
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

fn merge_openmw_cfg_overlay(base: &str, resolved: &ResolvedOpenMwConfig) -> String {
    let mut out = String::from(NEREVAR_OVERLAY_HEADER);
    out.push_str(&format!("encoding={}\n", resolved.encoding));
    for path in &resolved.data_paths {
        out.push_str(&format!("data={path}\n"));
    }
    for plugin in &resolved.content {
        out.push_str(&format!("content={plugin}\n"));
    }
    out.push('\n');

    for line in base.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(NEREVAR_OVERLAY_HEADER.trim()) {
            continue;
        }
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }
        if trimmed.starts_with('#') {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let key = trimmed.split('=').next().unwrap_or("").trim();
        if key.eq_ignore_ascii_case("data")
            || key.eq_ignore_ascii_case("content")
            || key.eq_ignore_ascii_case("encoding")
        {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }

    out
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
    fn writes_openmw_cfg_in_launch_directory() {
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

    #[test]
    fn merge_writes_full_load_order_to_tes3mp_cfg() {
        let base = r#"# shipped defaults
data="?global?data"
data=./data
encoding=win1252
content=OldPlugin.esp
skip-menu=true
"#;
        let merged = merge_openmw_cfg_overlay(base, &sample_resolved());
        assert!(merged.contains("content=Better Bodies.esp"));
        assert!(merged.contains("content=Morrowind.esm"));
        assert!(!merged.contains("OldPlugin.esp"));
        assert!(!merged.contains("data=./data"));
        assert!(merged.contains("skip-menu=true"));
    }

    #[test]
    fn installs_overlay_next_to_tes3mp_exe() {
        let root = std::env::temp_dir().join(format!("nerevar-tes3mp-cfg-{}", std::process::id()));
        let data_dir = root.join("data");
        let tes3mp_dir = root.join("tes3mp");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&data_dir.join("Better Bodies")).unwrap();
        std::fs::create_dir_all(&tes3mp_dir).unwrap();
        std::fs::write(
            tes3mp_dir.join("openmw.cfg"),
            "data=./data\ncontent=Stale.esp\n",
        )
        .unwrap();

        let mut resolved = sample_resolved();
        resolved.data_paths = vec![
            "\"C:\\\\Morrowind\\\\Data Files\"".into(),
            format!("\"{}\\\\Better Bodies\"", data_dir.display()),
        ];

        let path = write_tes3mp_launch_openmw_cfg(&tes3mp_dir, &data_dir, &resolved).unwrap();
        assert_eq!(path, tes3mp_dir.join("openmw.cfg"));

        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("content=Better Bodies.esp"));
        assert!(contents.contains("content=Morrowind.esm"));
        assert!(!contents.contains("Stale.esp"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
