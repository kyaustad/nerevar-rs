//! The downloads folder of modlist installs, like Wabbajack's: one folder of archives for every instance, with an
//! index of where each came from. Installing a list again, repairing an instance or installing the same list into
//! another instance only downloads what changed or isn't there.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

const INDEX_FILE: &str = "nerevar-downloads.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexedFile {
    /// File name in the downloads folder
    file: String,
    size: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadIndex {
    /// Source of the file (`nexus:<file id>`, `gitlab:<id>`, a URL) -> its archive
    files: BTreeMap<String, IndexedFile>,
}

pub struct Downloads {
    dir: PathBuf,
    index: Mutex<DownloadIndex>,
}

/// Nerevar's downloads folder when the player hasn't chosen one.
pub fn default_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("Nerevar").join("downloads")
}

/// Where earlier versions of Nerevar kept archives: `<cache>/Nerevar/archives/<mod folder>/<file>`.
fn legacy_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("Nerevar").join("archives")
}

pub fn safe_file_name(name: &str) -> String {
    let cleaned: String =
        name.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    let cleaned = cleaned.trim().trim_end_matches('.').to_string();
    if cleaned.is_empty() { "download".into() } else { cleaned }
}

impl Downloads {
    pub fn open(dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
        let index = std::fs::read_to_string(dir.join(INDEX_FILE))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Ok(Self { dir, index: Mutex::new(index) })
    }

    fn save(&self, index: &DownloadIndex) -> Result<(), String> {
        let json = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
        let path = self.dir.join(INDEX_FILE);
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&temp, &path).map_err(|e| e.to_string())
    }

    /// The archive of `key`, when it's there with the expected size (any size when it isn't known).
    pub fn cached(&self, key: &str, size: Option<u64>) -> Option<PathBuf> {
        let index = self.index.lock().ok()?;
        let entry = index.files.get(key)?;
        let path = self.dir.join(&entry.file);
        let len = std::fs::metadata(&path).ok()?.len();
        (len == entry.size && size.is_none_or(|size| size == len)).then_some(path)
    }

    /// Takes an archive an earlier Nerevar downloaded for `mod_dir` into the downloads folder.
    pub fn adopt_legacy(&self, key: &str, mod_dir: &str, file_name: &str, size: Option<u64>) -> Option<PathBuf> {
        let old = legacy_dir().join(safe_file_name(mod_dir)).join(safe_file_name(file_name));
        let len = std::fs::metadata(&old).ok()?.len();
        if size.is_some_and(|size| size != len) {
            return None;
        }
        let target = self.path_for(key, file_name);
        if std::fs::rename(&old, &target).is_err() {
            std::fs::copy(&old, &target).ok()?;
            let _ = std::fs::remove_file(&old);
        }
        self.record(key, &target).ok()?;
        Some(target)
    }

    /// Where to download the file of `key`: its earlier file, else a new name that no other source uses.
    pub fn path_for(&self, key: &str, file_name: &str) -> PathBuf {
        let Ok(index) = self.index.lock() else {
            return self.dir.join(safe_file_name(file_name));
        };
        if let Some(entry) = index.files.get(key) {
            return self.dir.join(&entry.file);
        }
        let name = safe_file_name(file_name);
        let taken = |candidate: &str| {
            index.files.values().any(|f| f.file.eq_ignore_ascii_case(candidate)) || self.dir.join(candidate).exists()
        };
        if !taken(&name) {
            return self.dir.join(name);
        }
        let (stem, ext) = match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), format!(".{ext}")),
            _ => (name.clone(), String::new()),
        };
        (2..).map(|n| format!("{stem} ({n}){ext}")).find(|candidate| !taken(candidate)).map(|c| self.dir.join(c)).unwrap()
    }

    /// Remembers that `path` (in the downloads folder) is the archive of `key`.
    pub fn record(&self, key: &str, path: &Path) -> Result<(), String> {
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        let file = path.file_name().ok_or("Invalid download path")?.to_string_lossy().into_owned();
        let mut index = self.index.lock().map_err(|_| "Downloads index lock poisoned".to_string())?;
        index.files.insert(key.to_string(), IndexedFile { file, size });
        self.save(&index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuses_and_names_downloads() {
        let dir = std::env::temp_dir().join(format!("nerevar-downloads-{}", uuid::Uuid::new_v4()));
        let downloads = Downloads::open(dir.clone()).unwrap();
        let first = downloads.path_for("nexus:1", "Mod.7z");
        std::fs::write(&first, b"12345").unwrap();
        downloads.record("nexus:1", &first).unwrap();
        assert_eq!(downloads.cached("nexus:1", Some(5)), Some(first.clone()));
        assert_eq!(downloads.cached("nexus:1", Some(6)), None);
        assert_eq!(downloads.cached("nexus:2", None), None);

        // Another source with the same file name gets its own file
        let second = downloads.path_for("nexus:2", "Mod.7z");
        assert_eq!(second.file_name().unwrap(), "Mod (2).7z");

        // The index survives reopening
        let reopened = Downloads::open(dir.clone()).unwrap();
        assert_eq!(reopened.cached("nexus:1", None), Some(first));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
