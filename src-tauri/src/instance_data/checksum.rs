use std::fmt::Write as _;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Deterministic SHA-256 over all files in `dir` (relative path + contents), for change detection.
pub fn directory_tree_checksum(dir: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(dir, dir, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut tree = Sha256::new();
    for (relative, path) in files {
        let mut file_hasher = Sha256::new();
        file_hasher.update(relative.as_bytes());
        file_hasher.update([0]);

        let mut file = File::open(&path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
        let mut buffer = [0u8; 8192];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
            if read == 0 {
                break;
            }
            file_hasher.update(&buffer[..read]);
        }

        tree.update(file_hasher.finalize());
    }

    Ok(format!("sha256:{}", hex_encode(tree.finalize())))
}

pub fn file_checksum(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{}", hex_encode(hasher.finalize())))
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

pub fn collect_package_files(
    package_dir: &Path,
) -> Result<Vec<super::types::ManifestFileEntry>, String> {
    let mut files = Vec::new();
    let mut paths = Vec::new();
    collect_files(package_dir, package_dir, &mut paths)?;
    paths.sort_by(|a, b| a.0.cmp(&b.0));

    for (relative, path) in paths {
        let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        files.push(super::types::ManifestFileEntry {
            path: relative,
            size: metadata.len(),
            checksum: file_checksum(&path)?,
        });
    }

    Ok(files)
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
