use std::path::Path;

/// Classic TES3 record plugins sorted by master dependencies.
pub const RECORD_PLUGIN_EXTENSIONS: &[&str] = &["esm", "esp"];

/// Additional OpenMW `content=` entries (Lua packs, archives, etc.).
pub const EXTENDED_CONTENT_EXTENSIONS: &[&str] = &["omwscripts", "omwaddon", "bsa"];

pub fn file_extension(name: &str) -> Option<&str> {
    Path::new(name).extension()?.to_str()
}

pub fn is_record_plugin(name: &str) -> bool {
    file_extension(name)
        .is_some_and(|ext| RECORD_PLUGIN_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
}

pub fn is_openmw_content_file(name: &str) -> bool {
    file_extension(name).is_some_and(|ext| is_content_extension(ext))
}

pub fn is_openmw_content_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(is_content_extension)
}

fn is_content_extension(ext: &str) -> bool {
    RECORD_PLUGIN_EXTENSIONS
        .iter()
        .chain(EXTENDED_CONTENT_EXTENSIONS.iter())
        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_openmw_content_types() {
        assert!(is_openmw_content_file("TR_Mainland.esm"));
        assert!(is_openmw_content_file("Patch.esp"));
        assert!(is_openmw_content_file("my_mod.omwscripts"));
        assert!(is_openmw_content_file("scripts.omwaddon"));
        assert!(is_openmw_content_file("TR_Mainland.bsa"));
        assert!(!is_openmw_content_file("readme.txt"));
    }

    #[test]
    fn record_plugins_exclude_lua_and_archives() {
        assert!(is_record_plugin("TR_Mainland.esm"));
        assert!(!is_record_plugin("my_mod.omwscripts"));
        assert!(!is_record_plugin("TR_Mainland.bsa"));
    }
}
