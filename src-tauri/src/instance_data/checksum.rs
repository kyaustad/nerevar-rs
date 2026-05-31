use std::fmt::Write as _;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use sha2::{Digest, Sha256};

use super::progress::{BackgroundOperationPhase, ProgressEmitter};
use super::types::ManifestFileEntry;

pub struct PackageHashResult {
    pub tree_checksum: String,
    pub files: Vec<ManifestFileEntry>,
    pub total_size_bytes: u64,
}

struct HashedFile {
    relative: String,
    size: u64,
    content_checksum: String,
    tree_leaf: [u8; 32],
}

/// Hash every file in a package directory once, producing both the tree fingerprint and
/// per-file manifest entries.
pub fn hash_package_directory(
    package_dir: &Path,
    progress: Option<Arc<Mutex<ProgressEmitter>>>,
) -> Result<PackageHashResult, String> {
    let mut paths = Vec::new();
    collect_files(package_dir, package_dir, &mut paths)?;
    paths.sort_by(|a, b| a.0.cmp(&b.0));

    let total = paths.len() as u64;
    let hashed: Result<Vec<HashedFile>, String> = paths
        .par_iter()
        .enumerate()
        .map(|(index, (relative, path))| {
            if let Some(emitter) = progress.as_ref() {
                if let Ok(mut guard) = emitter.lock() {
                    guard.emit(
                        BackgroundOperationPhase::HashingFiles,
                        format!("Hashing files ({}/{})", index + 1, total),
                        index as u64 + 1,
                        total,
                        Some(relative.clone()),
                        false,
                    );
                }
            }
            hash_file_single_pass(relative, path)
        })
        .collect();

    let mut hashed = hashed?;
    hashed.sort_by(|a, b| a.relative.cmp(&b.relative));

    let mut tree = Sha256::new();
    let mut files = Vec::with_capacity(hashed.len());
    let mut total_size_bytes = 0u64;

    for entry in hashed {
        tree.update(entry.tree_leaf);
        total_size_bytes += entry.size;
        files.push(ManifestFileEntry {
            path: entry.relative,
            size: entry.size,
            checksum: entry.content_checksum,
        });
    }

    Ok(PackageHashResult {
        tree_checksum: format!("sha256:{}", hex_encode(tree.finalize())),
        files,
        total_size_bytes,
    })
}

/// Deterministic SHA-256 over all files in `dir` (relative path + contents), for change detection.
#[cfg(test)]
pub fn directory_tree_checksum(
    dir: &Path,
    progress: &mut Option<ProgressEmitter>,
) -> Result<String, String> {
    let shared = progress.take().map(|emitter| Arc::new(Mutex::new(emitter)));
    let result = hash_package_directory(dir, shared)?;
    Ok(result.tree_checksum)
}

#[cfg(test)]
pub fn file_checksum(path: &Path) -> Result<String, String> {
    let relative = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string();
    Ok(hash_file_single_pass(&relative, path)?.content_checksum)
}

fn hash_file_single_pass(relative: &str, path: &Path) -> Result<HashedFile, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let mut file = File::open(path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;

    let mut tree_leaf_hasher = Sha256::new();
    tree_leaf_hasher.update(relative.as_bytes());
    tree_leaf_hasher.update([0u8]);

    let mut content_hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        tree_leaf_hasher.update(chunk);
        content_hasher.update(chunk);
    }

    Ok(HashedFile {
        relative: relative.to_string(),
        size: metadata.len(),
        content_checksum: format!("sha256:{}", hex_encode(content_hasher.finalize())),
        tree_leaf: tree_leaf_hasher.finalize().into(),
    })
}

fn collect_files(
    root: &Path,
    current: &Path,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(current)
        .map_err(|e| format!("Failed to read directory {}: {e}", current.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "Path prefix error".to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            out.push((relative, path));
        }
    }

    Ok(())
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .fold(String::new(), |mut acc, b| {
            write!(acc, "{b:02x}").unwrap();
            acc
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hash_package_matches_legacy_tree_and_file_checksums() {
        let dir = std::env::temp_dir().join(format!("nerevar-hash-package-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("a.esp"), b"alpha").unwrap();
        fs::write(dir.join("nested/b.esp"), b"beta").unwrap();

        let package = hash_package_directory(&dir, None).unwrap();
        let legacy_tree = {
            let mut progress = None;
            directory_tree_checksum(&dir, &mut progress).unwrap()
        };

        assert_eq!(package.tree_checksum, legacy_tree);
        assert_eq!(package.files.len(), 2);
        assert_eq!(package.total_size_bytes, 10);

        for file in &package.files {
            let path = dir.join(&file.path);
            assert_eq!(file.checksum, file_checksum(&path).unwrap());
        }

        let _ = fs::remove_dir_all(&dir);
    }
}
