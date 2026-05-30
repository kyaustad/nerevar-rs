use std::path::Path;

use uuid::Uuid;

use super::paths::{INSTANCE_DATA_DIR, NEREVAR_DIR};
use crate::instance_setup::INSTANCE_TES3MP_DIR;
use crate::openmw_ini_importer::is_openmw_content_path;
use super::types::{PackageKind, ScannedPackage};

const DATA_DIR_NAMES: &[&str] = &[
    "meshes", "textures", "icons", "music", "sound", "bookart", "fonts", "video",
];

/// Subdirectories that never contain OpenMW `content=` files. Skipping them keeps
/// plugin discovery fast on large mods (Tamriel Rebuilt, BCOM, etc.).
const PLUGIN_SEARCH_SKIP_DIRS: &[&str] = &[
    "meshes",
    "textures",
    "icons",
    "music",
    "sound",
    "bookart",
    "fonts",
    "video",
    "distantland",
    "shaders",
    "screenshots",
    "fanim",
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

        packages.push(ScannedPackage {
            name: folder_name.clone(),
            kind,
            relative_dir: folder_name,
            plugins,
            tree_checksum: None,
        });
    }

    packages.sort_by(|a, b| a.relative_dir.cmp(&b.relative_dir));
    Ok(packages)
}

pub fn should_skip_package_dir(name: &str) -> bool {
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
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if should_skip_plugin_search_dir(name) {
                continue;
            }
            collect_plugins(&path, out);
            continue;
        }
        if !path.is_file() {
            continue;
        }
        if !is_openmw_content_path(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            out.push(name.to_string());
        }
    }
}

fn should_skip_plugin_search_dir(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    PLUGIN_SEARCH_SKIP_DIRS
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
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
        assert!(bb.tree_checksum.is_none());

        let rr = packages
            .iter()
            .find(|p| p.name == "Rock Replacer")
            .expect("Rock Replacer");
        assert!(matches!(rr.kind, PackageKind::Replacer));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_finds_nested_openmw_content_files() {
        let dir = std::env::temp_dir().join(format!("nerevar-scan-ext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ensure_instance_data_layout(&dir).unwrap();

        let lua_mod = dir.join("Lua Pack");
        std::fs::create_dir_all(lua_mod.join("scripts")).unwrap();
        std::fs::write(lua_mod.join("scripts/pack.omwscripts"), b"GLOBAL: scripts/x.lua").unwrap();

        let rebuilt = dir.join("Tamriel Rebuilt");
        std::fs::create_dir_all(rebuilt.join("meshes")).unwrap();
        std::fs::write(rebuilt.join("TR_Mainland.esm"), b"TES3").unwrap();
        std::fs::write(rebuilt.join("TR_Mainland.bsa"), b"BSA").unwrap();

        let packages = scan_data_directory(&dir).unwrap();
        let lua = packages.iter().find(|p| p.name == "Lua Pack").expect("Lua Pack");
        assert!(lua
            .plugins
            .iter()
            .any(|p| p.eq_ignore_ascii_case("pack.omwscripts")));

        let tr = packages
            .iter()
            .find(|p| p.name == "Tamriel Rebuilt")
            .expect("Tamriel Rebuilt");
        assert!(tr.plugins.iter().any(|p| p.eq_ignore_ascii_case("TR_Mainland.esm")));
        assert!(tr.plugins.iter().any(|p| p.eq_ignore_ascii_case("TR_Mainland.bsa")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plugin_search_skips_asset_directories() {
        let dir = std::env::temp_dir().join(format!("nerevar-scan-skip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("meshes/deep")).unwrap();
        std::fs::write(dir.join("meshes/deep/hidden.esp"), b"").unwrap();
        std::fs::write(dir.join("visible.esp"), b"").unwrap();

        let plugins = find_plugins(&dir);
        assert!(plugins.iter().any(|p| p.eq_ignore_ascii_case("visible.esp")));
        assert!(!plugins.iter().any(|p| p.eq_ignore_ascii_case("hidden.esp")));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
