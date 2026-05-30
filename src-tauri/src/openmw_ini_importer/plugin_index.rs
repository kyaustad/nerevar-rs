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
#[derive(Debug, Default, Clone)]
pub struct PluginIndex {
    by_name: HashMap<String, PathBuf>,
}

impl PluginIndex {
    pub fn build(data_paths: &[PathBuf]) -> Self {
        if data_paths.is_empty() {
            return Self::default();
        }

        if data_paths.len() == 1 {
            let mut by_name = HashMap::new();
            index_data_path(&data_paths[0], &mut by_name);
            return Self { by_name };
        }

        let partials: Vec<HashMap<String, PathBuf>> = data_paths
            .par_iter()
            .map(|data_path| {
                let mut local = HashMap::new();
                index_data_path(data_path, &mut local);
                local
            })
            .collect();

        let mut by_name = HashMap::new();
        for partial in partials {
            by_name.extend(partial);
        }
        Self { by_name }
    }

    pub fn find(&self, plugin_name: &str) -> Option<&PathBuf> {
        self.by_name.get(&plugin_name.to_ascii_lowercase())
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }
}

fn index_data_path(data_path: &Path, out: &mut HashMap<String, PathBuf>) {
    if !data_path.is_dir() {
        return;
    }
    index_directory(data_path, out);
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
}
