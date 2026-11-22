//! Extracting downloaded mods: zip, 7z and rar archives, or single files such as plugins.

use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

enum Format {
    Zip,
    SevenZip,
    Rar,
    Plain,
}

fn detect(path: &Path) -> Result<Format, String> {
    let mut magic = [0u8; 8];
    let read = File::open(path)
        .and_then(|mut file| file.read(&mut magic))
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let magic = &magic[..read];
    Ok(if magic.starts_with(b"PK\x03\x04") || magic.starts_with(b"PK\x05\x06") {
        Format::Zip
    } else if magic.starts_with(&[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C]) {
        Format::SevenZip
    } else if magic.starts_with(b"Rar!\x1A\x07") {
        Format::Rar
    } else {
        Format::Plain
    })
}

/// Extracts `archive` into `dest`, which is created. A file that isn't an archive is copied into `dest` with the
/// name `file_name`.
pub fn extract(archive: &Path, dest: &Path, file_name: &str) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("Failed to create {}: {e}", dest.display()))?;
    match detect(archive)? {
        Format::Zip => {
            let file = File::open(archive).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip {}: {e}", archive.display()))?;
            // Entries with unsafe paths are skipped by enclosed_name
            for index in 0..zip.len() {
                let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
                let Some(relative) = entry.enclosed_name() else { continue };
                let out = dest.join(relative);
                if entry.is_dir() {
                    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                    continue;
                }
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut target = File::create(&out).map_err(|e| format!("Failed to write {}: {e}", out.display()))?;
                std::io::copy(&mut entry, &mut target).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        Format::SevenZip => sevenz_rust2::decompress_file(archive, dest)
            .map_err(|e| format!("Failed to extract {}: {e}", archive.display())),
        Format::Rar => extract_rar(archive, dest),
        Format::Plain => {
            let name = Path::new(file_name)
                .file_name()
                .ok_or_else(|| format!("Invalid file name {file_name}"))?;
            std::fs::copy(archive, dest.join(name))
                .map(|_| ())
                .map_err(|e| format!("Failed to copy {}: {e}", archive.display()))
        }
    }
}

fn extract_rar(archive: &Path, dest: &Path) -> Result<(), String> {
    let mut cursor = unrar::Archive::new(archive)
        .open_for_processing()
        .map_err(|e| format!("Invalid rar {}: {e}", archive.display()))?;
    while let Some(header) = cursor.read_header().map_err(|e| format!("Failed to read {}: {e}", archive.display()))? {
        let entry = header.entry();
        let safe = entry.filename.components().all(|part| matches!(part, Component::Normal(_)));
        cursor = if entry.is_file() && safe {
            header.extract_with_base(dest).map_err(|e| format!("Failed to extract {}: {e}", archive.display()))?
        } else {
            header.skip().map_err(|e| e.to_string())?
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extracts_zips_and_copies_plain_files() {
        let root = std::env::temp_dir().join(format!("nerevar-archive-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let zip_path = root.join("mod.zip");
        {
            let mut writer = zip::ZipWriter::new(File::create(&zip_path).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("Data Files/mod.esp", options).unwrap();
            writer.write_all(b"plugin").unwrap();
            writer.finish().unwrap();
        }
        extract(&zip_path, &root.join("out"), "mod.zip").unwrap();
        assert_eq!(std::fs::read(root.join("out/Data Files/mod.esp")).unwrap(), b"plugin");

        let plain = root.join("download.tmp");
        std::fs::write(&plain, b"esp").unwrap();
        extract(&plain, &root.join("single"), "Patch.esp").unwrap();
        assert_eq!(std::fs::read(root.join("single/Patch.esp")).unwrap(), b"esp");
        let _ = std::fs::remove_dir_all(&root);
    }
}
