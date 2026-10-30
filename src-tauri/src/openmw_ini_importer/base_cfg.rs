//! The base OpenMW config generated from the player's Morrowind install.
//!
//! TES3MP-NG installs are portable, so Nerevar no longer swaps `Documents/My Games/OpenMW/openmw.cfg`
//! around launches. Onboarding imports `Morrowind.ini` once (encoding, `Data Files`, archives and
//! fallback values) into `openmw.base.cfg` in Nerevar's app data folder, and each instance's own
//! `openmw.cfg` is generated from it, so players never need OpenMW's wizard.

use std::fs;
use std::path::{Path, PathBuf};

use super::importer::{
    apply_morrowind_ini_import, build_default_morrowind_ini, load_cfg_file, quote_data_path,
    resolve_morrowind_ini, ImportOptions, IniEncoding, MultiStrMap,
};

pub const OPENMW_BASE_CFG: &str = "openmw.base.cfg";

pub fn base_openmw_cfg_path() -> Result<PathBuf, String> {
    let config = crate::config::nerevar_config_file_path()?;
    let dir = config
        .parent()
        .ok_or_else(|| "Failed to resolve Nerevar's app data folder".to_string())?;
    Ok(dir.join(OPENMW_BASE_CFG))
}

/// Whether onboarding generated the base config.
pub fn validate_base_openmw_cfg() -> Result<bool, String> {
    validate_base_openmw_cfg_at(&base_openmw_cfg_path()?)
}

fn validate_base_openmw_cfg_at(path: &Path) -> Result<bool, String> {
    if !path.is_file() {
        return Ok(false);
    }
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    Ok(contents.contains("data="))
}

/// Import the Morrowind install at `morrowind_data_files` (its `Data Files` folder).
pub fn setup_base_openmw_cfg(morrowind_data_files: &Path) -> Result<(), String> {
    setup_base_openmw_cfg_at(&base_openmw_cfg_path()?, morrowind_data_files)
}

fn setup_base_openmw_cfg_at(path: &Path, morrowind_data_files: &Path) -> Result<(), String> {
    if !morrowind_data_files.join("Morrowind.esm").is_file() {
        return Err(format!(
            "Morrowind.esm not found in {}",
            morrowind_data_files.display()
        ));
    }

    let ini = match resolve_morrowind_ini(morrowind_data_files) {
        Some(morrowind_ini) => super::importer::load_ini_file(&morrowind_ini, IniEncoding::Win1252)?,
        None => build_default_morrowind_ini(morrowind_data_files),
    };

    let mut seed = MultiStrMap::new();
    seed.insert("encoding".to_string(), vec!["win1252".to_string()]);
    seed.insert(
        "data".to_string(),
        vec![quote_data_path(morrowind_data_files)],
    );

    // Start from scratch: a new Morrowind install replaces the previous import
    if path.is_file() {
        fs::remove_file(path).map_err(|e| format!("Failed to replace {}: {e}", path.display()))?;
    }
    apply_morrowind_ini_import(
        &ini,
        path,
        seed,
        ImportOptions {
            encoding: IniEncoding::Win1252,
            import_game_files: false,
            import_archives: true,
        },
        morrowind_data_files,
    )
}

pub fn load_base_openmw_cfg() -> Result<MultiStrMap, String> {
    let path = base_openmw_cfg_path()?;
    if !path.is_file() {
        return Err(
            "Nerevar's Morrowind setup is missing. Pick your Morrowind install in onboarding first."
                .to_string(),
        );
    }
    load_cfg_file(&path)
}

/// Morrowind's `Data Files` folder from the base config (its first `data=` entry).
pub fn read_base_data_path() -> Option<String> {
    let cfg = load_cfg_file(&base_openmw_cfg_path().ok()?).ok()?;
    cfg.get("data").and_then(|values| values.first().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_a_morrowind_install() {
        let dir = std::env::temp_dir().join(format!("nerevar-base-cfg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let data_files = dir.join("Morrowind/Data Files");
        fs::create_dir_all(&data_files).unwrap();
        fs::write(data_files.join("Morrowind.esm"), b"TES3").unwrap();
        fs::write(
            dir.join("Morrowind/Morrowind.ini"),
            "[Weather]\r\nSunrise Time=6\r\n[Archives]\r\nArchive 0=Tribunal.bsa\r\n",
        )
        .unwrap();
        let path = dir.join(OPENMW_BASE_CFG);

        assert!(!validate_base_openmw_cfg_at(&path).unwrap());
        setup_base_openmw_cfg_at(&path, &data_files).unwrap();
        assert!(validate_base_openmw_cfg_at(&path).unwrap());

        let cfg = load_cfg_file(&path).unwrap();
        assert_eq!(cfg.get("encoding").unwrap(), &vec!["win1252".to_string()]);
        assert!(cfg.get("data").unwrap()[0].contains("Data Files"));
        assert!(cfg.get("fallback-archive").is_some_and(|a| !a.is_empty()));
        assert!(cfg.get("fallback").is_some_and(|f| !f.is_empty()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_folders_without_morrowind() {
        let dir = std::env::temp_dir().join(format!("nerevar-base-cfg-bad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(setup_base_openmw_cfg_at(&dir.join(OPENMW_BASE_CFG), &dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
