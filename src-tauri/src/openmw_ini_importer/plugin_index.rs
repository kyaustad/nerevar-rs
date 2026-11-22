use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use super::content_files::is_openmw_content_path;

/// Directories that never contain OpenMW `content=` files (matches instance scan skips).
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

pub fn should_skip_plugin_search_dir(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    PLUGIN_SEARCH_SKIP_DIRS
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

/// Case-insensitive plugin filename → absolute path on disk.
///
/// Built with one walk per `data=` path. Later paths in the slice override earlier
/// entries, matching OpenMW / `find_plugin_in_data_paths` priority (last wins).
/// OpenMW only loads content files from the top level of a data folder, so a top-level copy
/// in any data path wins over copies in subfolders (mod managers keep alternatives in folders
/// like `optional/`); subfolders are only searched for plugins no data path has at its top.
#[derive(Debug, Default, Clone)]
pub struct PluginIndex {
    by_name: HashMap<String, PathBuf>,
}

/// One data path's plugins: those at its top level, and those in its subfolders.
#[derive(Default)]
struct DataPathPlugins {
    top: HashMap<String, PathBuf>,
    nested: HashMap<String, PathBuf>,
}

impl PluginIndex {
    pub fn build(data_paths: &[PathBuf]) -> Self {
        let partials: Vec<DataPathPlugins> = data_paths
            .par_iter()
            .map(|data_path| {
                let mut local = DataPathPlugins::default();
                index_data_path(data_path, &mut local);
                local
            })
            .collect();

        let mut top = HashMap::new();
        let mut nested = HashMap::new();
        for partial in partials {
            top.extend(partial.top);
            nested.extend(partial.nested);
        }
        nested.extend(top);
        Self { by_name: nested }
    }

    pub fn find(&self, plugin_name: &str) -> Option<&PathBuf> {
        self.by_name.get(&plugin_name.to_ascii_lowercase())
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }
}

fn index_data_path(data_path: &Path, out: &mut DataPathPlugins) {
    if !data_path.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(data_path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !should_skip_plugin_search_dir(name) {
                index_directory(&path, &mut out.nested);
            }
        } else if path.is_file() && is_openmw_content_path(&path) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                out.top.insert(name.to_ascii_lowercase(), path);
            }
        }
    }
}

fn index_directory(dir: &Path, out: &mut HashMap<String, PathBuf>) {
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
            index_directory(&path, out);
            continue;
        }

        if !path.is_file() || !is_openmw_content_path(&path) {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        out.insert(name.to_ascii_lowercase(), path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn index_finds_nested_plugins_and_skips_asset_dirs() {
        let dir = std::env::temp_dir().join(format!("nerevar-plugin-index-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("meshes/deep")).unwrap();
        fs::write(dir.join("meshes/deep/hidden.esp"), b"").unwrap();
        fs::create_dir_all(dir.join("scripts")).unwrap();
        fs::write(dir.join("scripts/pack.omwscripts"), b"x").unwrap();
        fs::write(dir.join("Root.esp"), b"").unwrap();

        let index = PluginIndex::build(&[dir.clone()]);
        assert!(index.find("Root.esp").is_some());
        assert!(index.find("pack.omwscripts").is_some());
        assert!(index.find("hidden.esp").is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn later_data_path_overrides_earlier() {
        let dir = std::env::temp_dir().join(format!("nerevar-plugin-priority-{}", std::process::id()));
        let low = dir.join("low");
        let high = dir.join("high");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&low).unwrap();
        fs::create_dir_all(&high).unwrap();
        fs::write(low.join("Clash.esp"), b"low").unwrap();
        fs::write(high.join("Clash.esp"), b"high").unwrap();

        let index = PluginIndex::build(&[low.clone(), high.clone()]);
        assert_eq!(
            index.find("Clash.esp").map(|path| path.parent().unwrap()),
            Some(high.as_path())
        );

        let _ = fs::remove_dir_all(&dir);
    }

    /// OpenMW loads the top-level copy; an alternative a mod manager keeps in a later data path's
    /// `optional/` folder must not win (its checksum made the server reject the client).
    #[test]
    fn top_level_copies_win_over_nested_ones() {
        let dir = std::env::temp_dir().join(format!("nerevar-plugin-top-{}", std::process::id()));
        let low = dir.join("low");
        let high = dir.join("high");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&low).unwrap();
        fs::create_dir_all(high.join("optional")).unwrap();
        fs::write(low.join("Clash.esp"), b"loaded").unwrap();
        fs::write(high.join("optional").join("Clash.esp"), b"alternative").unwrap();
        fs::write(high.join("optional").join("OnlyNested.esp"), b"x").unwrap();

        let index = PluginIndex::build(&[low.clone(), high.clone()]);
        assert_eq!(index.find("Clash.esp").map(|path| path.parent().unwrap()), Some(low.as_path()));
        assert!(index.find("OnlyNested.esp").is_some());

        let _ = fs::remove_dir_all(&dir);
    }
}
