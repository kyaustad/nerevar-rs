//! Shader packages: finding them among an instance's packages, and installing core shader packages.
//!
//! Post-processing packages (`shaders/*.omwfx`) are data folders like any other, since OpenMW loads them
//! through its VFS. Core shader packages (`resources/shaders`, like PBR shaders) replace OpenMW's own
//! shaders, which OpenMW only reads from its install: they are copied over the install's
//! `resources/shaders`, and the files they replaced are kept, so they come back when the package is
//! disabled or removed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::instance_data::{package_abs_path, LoadOrder, ShaderPackageKind};

const POST_PROCESSING_EXTENSION: &str = "omwfx";
const CORE_SHADER_EXTENSIONS: &[&str] = &["glsl", "frag", "vert", "geom", "comp"];
/// Plugins that make a package a mod, rather than only shaders. Lua scripts (`.omwscripts`) are fine:
/// post-processing packages use them to drive their shaders.
const DATA_PLUGIN_EXTENSIONS: &[&str] = &["esm", "esp", "omwaddon", "omwgame"];

const STATE_FILE: &str = ".nerevar-core-shaders.json";
const BACKUP_DIR: &str = ".nerevar-core-shaders-backup";

/// What kind of shader package a package folder is, if it is one.
pub fn detect_shader_kind(package_dir: &Path, plugins: &[String]) -> Option<ShaderPackageKind> {
    let has_data_plugins = plugins.iter().any(|plugin| {
        Path::new(plugin)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| DATA_PLUGIN_EXTENSIONS.iter().any(|data| ext.eq_ignore_ascii_case(data)))
    });
    if has_data_plugins {
        return None;
    }
    if core_shaders_dir(package_dir).is_some() {
        return Some(ShaderPackageKind::CoreShaders);
    }
    let shaders = find_child_dir(package_dir, "shaders")?;
    has_file_with_extension(&shaders, &[POST_PROCESSING_EXTENSION]).then_some(ShaderPackageKind::PostProcessing)
}

/// The folder of a core shader package that mirrors the install's `resources/shaders`: its
/// `resources/shaders`, or a `shaders` folder of OpenMW shader sources without post-processing shaders.
pub fn core_shaders_dir(package_dir: &Path) -> Option<PathBuf> {
    if let Some(dir) = find_child_dir(package_dir, "resources").and_then(|resources| find_child_dir(&resources, "shaders")) {
        if has_file_with_extension(&dir, CORE_SHADER_EXTENSIONS) {
            return Some(dir);
        }
    }
    let dir = find_child_dir(package_dir, "shaders")?;
    (has_file_with_extension(&dir, CORE_SHADER_EXTENSIONS) && !has_file_with_extension(&dir, &[POST_PROCESSING_EXTENSION]))
        .then_some(dir)
}

/// The core shader folders of the load order's enabled core shader packages, lowest priority first.
pub fn enabled_core_shader_dirs(data_dir: &Path, load_order: &LoadOrder) -> Vec<PathBuf> {
    let mut entries: Vec<_> = load_order
        .entries
        .iter()
        .filter(|entry| entry.enabled && entry.is_core_shaders())
        .collect();
    entries.sort_by_key(|entry| entry.priority);
    entries
        .into_iter()
        .filter_map(|entry| core_shaders_dir(&package_abs_path(data_dir, &entry.relative_dir)))
        .collect()
}

/// A file Nerevar wrote into the install's shaders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct InstalledFile {
    /// Checksum of what Nerevar wrote. A different file means the release was reinstalled since.
    checksum: String,
    /// Whether the install had its own file there, kept in the backup folder.
    replaced_original: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledState {
    files: BTreeMap<String, InstalledFile>,
}

/// Make the install's `resources/shaders` the release's shaders with `sources` copied over them, later
/// folders replacing earlier ones. Files of packages no longer listed get the release's file back.
/// Returns how many files are replaced.
pub fn apply_core_shaders(tes3mp_dir: &Path, sources: &[PathBuf]) -> Result<usize, String> {
    let resources = tes3mp_dir.join("resources");
    let shaders = resources.join("shaders");
    let state_path = resources.join(STATE_FILE);
    let backup = resources.join(BACKUP_DIR);
    if !shaders.is_dir() {
        // No release installed yet; this runs again on the next launch
        return Ok(0);
    }

    let mut state: InstalledState = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();

    let mut wanted: BTreeMap<String, PathBuf> = BTreeMap::new();
    for source in sources {
        for file in list_files(source) {
            if let Ok(relative) = file.strip_prefix(source) {
                wanted.insert(relative_key(relative), file.clone());
            }
        }
    }

    // Give back the release's files that are no longer replaced
    let stale: Vec<String> = state.files.keys().filter(|key| !wanted.contains_key(*key)).cloned().collect();
    for key in stale {
        let installed = state.files.remove(&key).expect("listed key");
        let target = shaders.join(&key);
        let backup_file = backup.join(&key);
        if file_checksum(&target).as_deref() == Some(installed.checksum.as_str()) {
            if installed.replaced_original && backup_file.is_file() {
                std::fs::copy(&backup_file, &target)
                    .map_err(|e| format!("Failed to restore {}: {e}", target.display()))?;
            } else {
                let _ = std::fs::remove_file(&target);
            }
        }
        let _ = std::fs::remove_file(&backup_file);
    }

    for (key, source) in &wanted {
        let target = shaders.join(key);
        let backup_file = backup.join(key);
        let source_checksum = file_checksum(source)
            .ok_or_else(|| format!("Failed to read {}", source.display()))?;
        let current = file_checksum(&target);

        let known = state.files.get(key).cloned();
        let ours = known.as_ref().is_some_and(|installed| current.as_deref() == Some(installed.checksum.as_str()));
        let replaced_original = if ours {
            known.map(|installed| installed.replaced_original).unwrap_or(false)
        } else if current.is_some() {
            // The release's own file (first install, or the release was reinstalled): keep it
            copy_file(&target, &backup_file)?;
            true
        } else {
            let _ = std::fs::remove_file(&backup_file);
            false
        };

        if current.as_deref() != Some(source_checksum.as_str()) {
            copy_file(source, &target)?;
        }
        state.files.insert(key.clone(), InstalledFile { checksum: source_checksum, replaced_original });
    }

    if state.files.is_empty() {
        let _ = std::fs::remove_file(&state_path);
        let _ = std::fs::remove_dir_all(&backup);
    } else {
        let json = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
        std::fs::write(&state_path, json)
            .map_err(|e| format!("Failed to write {}: {e}", state_path.display()))?;
    }
    Ok(state.files.len())
}

fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
    }
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("Failed to copy {} to {}: {e}", from.display(), to.display()))
}

fn file_checksum(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Lowercase with forward slashes: OpenMW's shader paths ignore case on Windows.
fn relative_key(relative: &Path) -> String {
    relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("/")
}

fn list_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else { continue };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn has_file_with_extension(dir: &Path, extensions: &[&str]) -> bool {
    list_files(dir).iter().any(|file| {
        file.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.iter().any(|wanted| ext.eq_ignore_ascii_case(wanted)))
    })
}

/// A child folder by name, ignoring case (mods ship `Shaders/` or `shaders/`).
pub fn find_child_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.is_dir() && path.file_name().is_some_and(|file| file.to_string_lossy().eq_ignore_ascii_case(name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nerevar-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn detects_shader_packages() {
        let root = temp_dir("shader-kinds");
        write(&root.join("XE/Shaders/hdr.omwfx"), "");
        write(&root.join("DoF/shaders/dof.omwfx"), "");
        write(&root.join("PBR/resources/shaders/compatibility/objects.frag"), "");
        write(&root.join("Mod/shaders/hdr.omwfx"), "");
        write(&root.join("Plain/textures/a.dds"), "");

        assert_eq!(detect_shader_kind(&root.join("XE"), &[]), Some(ShaderPackageKind::PostProcessing));
        assert_eq!(
            detect_shader_kind(&root.join("DoF"), &["DoF.omwscripts".into()]),
            Some(ShaderPackageKind::PostProcessing)
        );
        assert_eq!(detect_shader_kind(&root.join("PBR"), &[]), Some(ShaderPackageKind::CoreShaders));
        assert_eq!(detect_shader_kind(&root.join("Mod"), &["Mod.esp".into()]), None);
        assert_eq!(detect_shader_kind(&root.join("Plain"), &[]), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn installs_and_restores_core_shaders() {
        let root = temp_dir("core-shaders");
        let tes3mp = root.join("tes3mp");
        let shaders = tes3mp.join("resources/shaders");
        write(&shaders.join("compatibility/objects.frag"), "vanilla objects");
        write(&shaders.join("lib/light/lighting.glsl"), "vanilla lighting");
        let pbr = root.join("PBR/resources/shaders");
        write(&pbr.join("compatibility/objects.frag"), "pbr objects");
        write(&pbr.join("lib/light/lighting_pbr.glsl"), "pbr only");

        assert_eq!(apply_core_shaders(&tes3mp, &[pbr.clone()]).unwrap(), 2);
        assert_eq!(std::fs::read_to_string(shaders.join("compatibility/objects.frag")).unwrap(), "pbr objects");
        assert_eq!(std::fs::read_to_string(shaders.join("lib/light/lighting_pbr.glsl")).unwrap(), "pbr only");
        assert_eq!(std::fs::read_to_string(shaders.join("lib/light/lighting.glsl")).unwrap(), "vanilla lighting");

        // Applying again changes nothing
        assert_eq!(apply_core_shaders(&tes3mp, &[pbr.clone()]).unwrap(), 2);
        assert_eq!(std::fs::read_to_string(shaders.join("compatibility/objects.frag")).unwrap(), "pbr objects");

        // A reinstalled release puts its own file back: that becomes the file to restore
        write(&shaders.join("compatibility/objects.frag"), "new vanilla objects");
        apply_core_shaders(&tes3mp, &[pbr.clone()]).unwrap();
        assert_eq!(std::fs::read_to_string(shaders.join("compatibility/objects.frag")).unwrap(), "pbr objects");

        // Without the package, the release's shaders are back as they were
        assert_eq!(apply_core_shaders(&tes3mp, &[]).unwrap(), 0);
        assert_eq!(std::fs::read_to_string(shaders.join("compatibility/objects.frag")).unwrap(), "new vanilla objects");
        assert!(!shaders.join("lib/light/lighting_pbr.glsl").exists());
        assert!(!tes3mp.join("resources").join(STATE_FILE).exists());
        assert!(!tes3mp.join("resources").join(BACKUP_DIR).exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
