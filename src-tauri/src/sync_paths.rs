//! Shared path normalization and safety checks for sync file URLs.

/// Normalize a manifest-relative file path and reject traversal segments.
pub fn normalize_manifest_file_path(path: &str) -> Result<String, String> {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/').trim_end_matches('/');

    if trimmed.is_empty() {
        return Err("File path is empty".to_string());
    }

    for component in trimmed.split('/') {
        if component.is_empty() {
            return Err(format!("Invalid file path (empty segment): {path}"));
        }
        if component == "." || component == ".." {
            return Err(format!("Invalid file path (path traversal): {path}"));
        }
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_double_dots_inside_filename() {
        assert_eq!(
            normalize_manifest_file_path("textures/foo..bar.dds").unwrap(),
            "textures/foo..bar.dds"
        );
    }

    #[test]
    fn rejects_parent_directory_segments() {
        assert!(normalize_manifest_file_path("../secret.txt").is_err());
        assert!(normalize_manifest_file_path("mods/../secret.txt").is_err());
    }

    #[test]
    fn normalizes_backslashes() {
        assert_eq!(
            normalize_manifest_file_path("textures\\a.dds").unwrap(),
            "textures/a.dds"
        );
    }
}
