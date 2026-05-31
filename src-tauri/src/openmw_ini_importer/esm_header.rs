use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Master plugin dependencies listed in a TES3 plugin header.
pub fn read_master_dependencies(path: &Path) -> io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let mut rec_name = [0u8; 4];
    if file.read_exact(&mut rec_name).is_err() {
        return Ok(Vec::new());
    }
    if &rec_name != b"TES3" {
        return Ok(Vec::new());
    }

    let mut rec_size_buf = [0u8; 4];
    if file.read_exact(&mut rec_size_buf).is_err() {
        return Ok(Vec::new());
    }
    let rec_size = u32::from_le_bytes(rec_size_buf);
    let rec_end = file.stream_position()? + rec_size as u64;

    let mut masters = Vec::new();

    while file.stream_position()? < rec_end {
        let mut sub_name = [0u8; 4];
        file.read_exact(&mut sub_name)?;
        let mut sub_size_buf = [0u8; 4];
        file.read_exact(&mut sub_size_buf)?;
        let sub_size = u32::from_le_bytes(sub_size_buf) as usize;

        if &sub_name == b"FORM" {
            let mut format = [0u8; 4];
            file.read_exact(&mut format)?;
            continue;
        }

        if &sub_name == b"HEDR" {
            file.seek(SeekFrom::Current(sub_size as i64))?;
            continue;
        }

        if &sub_name == b"MAST" {
            let mut data = vec![0u8; sub_size];
            file.read_exact(&mut data)?;
            let name = read_zstring(&data);
            if !name.is_empty() {
                masters.push(name);
            }
            continue;
        }

        if &sub_name == b"DATA" {
            file.seek(SeekFrom::Current(sub_size as i64))?;
            continue;
        }

        // Remaining header subs (GMDT, SCRD, SCRS, ...) — skip.
        file.seek(SeekFrom::Current(sub_size as i64))?;
    }

    Ok(masters)
}

fn read_zstring(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
