use std::path::Path;

use uuid::Uuid;

use super::checksum::directory_tree_checksum;
use super::paths::{INSTANCE_DATA_DIR, NEREVAR_DIR};
use crate::instance_setup::INSTANCE_TES3MP_DIR;
use super::types::{PackageKind, ScannedPackage};

const DATA_DIR_NAMES: &[&str] = &[
    "meshes", "textures", "icons", "music", "sound", "bookart", "fonts", "video",
];

/// Scan immediate child folders of the instance data directory (e.g. `Better Bodies/`, `Rock Replacer/`).
pub fn scan_data_directory(data_dir: &Path) -> Result<Vec<ScannedPackage>, String> {
    let mut packages = Vec::new();

    if !data_dir.exists() {
        return Ok(packages);
    }

    let entries = std::fs::read_dir(data_dir)
        .map_err(|e| format!("Failed to read {}: {e}", data_dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = entry.file_name().to_string_lossy().to_string();

        if should_skip_package_dir(&folder_name) {
            continue;
        }

        let plugins = find_plugins(&path);
        let kind = classify_package(&path, &plugins);
        let tree_checksum = directory_tree_checksum(&path)?;

        packages.push(ScannedPackage {
            name: folder_name.clone(),
            kind,
            relative_dir: folder_name,
            plugins,
            tree_checksum,
        });
    }

    packages.sort_by(|a, b| a.relative_dir.cmp(&b.relative_dir));
    Ok(packages)
}

fn should_skip_package_dir(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    if name.eq_ignore_ascii_case(NEREVAR_DIR) {
        return true;
    }
    if name.eq_ignore_ascii_case(INSTANCE_TES3MP_DIR) {
        return true;
    }
    // Never treat the nested data folder as a package when scan root is wrong.
    name.eq_ignore_ascii_case(INSTANCE_DATA_DIR)
}

fn find_plugins(dir: &Path) -> Vec<String> {
    let mut plugins = Vec::new();
    collect_plugins(dir, &mut plugins);
    plugins.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    plugins.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    plugins
}

fn collect_plugins(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_plugins(&path, out);
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("esp") || ext.eq_ignore_ascii_case("esm") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                out.push(name.to_string());
            }
        }
    }
}

fn classify_package(dir: &Path, plugins: &[String]) -> PackageKind {
    if !plugins.is_empty() {
        return PackageKind::Mod;
    }
    if has_data_structure(dir) {
        return PackageKind::Replacer;
    }
    // Unknown empty-ish folder — treat as replacer so loose files still mount via data=.
    PackageKind::Replacer
}

fn has_data_structure(dir: &Path) -> bool {
    has_named_child_dir(dir, DATA_DIR_NAMES)
}

fn has_named_child_dir(dir: &Path, names: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if names
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
        {
            return true;
        }
    }
    false
}

pub fn new_entry_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::paths::ensure_instance_data_layout;

    #[test]
    fn scan_finds_top_level_packages() {
        let dir = std::env::temp_dir().join(format!("nerevar-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ensure_instance_data_layout(&dir).unwrap();

        let better = dir.join("Better Bodies");
        std::fs::create_dir_all(&better).unwrap();
        std::fs::write(better.join("betterbodies.esp"), b"").unwrap();

        let rock = dir.join("Rock Replacer");
        std::fs::create_dir_all(rock.join("textures")).unwrap();

        let packages = scan_data_directory(&dir).unwrap();
        assert_eq!(packages.len(), 2);

        let bb = packages
            .iter()
            .find(|p| p.name == "Better Bodies")
            .expect("Better Bodies");
        assert_eq!(bb.relative_dir, "Better Bodies");
        assert!(matches!(bb.kind, PackageKind::Mod));
        assert!(bb.plugins.iter().any(|p| p.eq_ignore_ascii_case("betterbodies.esp")));

        let rr = packages
            .iter()
            .find(|p| p.name == "Rock Replacer")
            .expect("Rock Replacer");
        assert!(matches!(rr.kind, PackageKind::Replacer));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
