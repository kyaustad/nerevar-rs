use std::fs;
use std::path::{Path, PathBuf};

use super::importer::{
    apply_morrowind_ini_import, build_default_morrowind_ini, cfg_to_string, load_cfg_file,
    parse_cfg_contents, quote_data_path, resolve_morrowind_ini, ImportOptions, IniEncoding,
    MultiStrMap,
};

pub const OPENMW_CFG: &str = "openmw.cfg";
pub const OPENMW_BACKUP_CFG: &str = "openmw.backup.cfg";
pub const OPENMW_NEREVAR_CFG: &str = "openmw.nerevar.cfg";

#[derive(Debug, Clone)]
pub struct OpenMwGlobalPaths {
    pub dir: PathBuf,
    pub active: PathBuf,
    pub backup: PathBuf,
    pub nerevar: PathBuf,
}

pub struct GlobalOpenMwLaunchSession {
    paths: OpenMwGlobalPaths,
    restore_strategy: OpenMwRestoreStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenMwRestoreStrategy {
    /// Restore `openmw.cfg` from `openmw.backup.cfg`.
    FromBackup,
    /// No pre-launch config or backup existed; remove the composed `openmw.cfg` we wrote.
    RemoveActive,
}

pub fn resolve_openmw_global_paths() -> Result<OpenMwGlobalPaths, String> {
    let documents_dir =
        dirs::document_dir().ok_or_else(|| "Failed to resolve documents directory".to_string())?;
    let dir = documents_dir.join("My Games/OpenMW");
    Ok(OpenMwGlobalPaths {
        active: dir.join(OPENMW_CFG),
        backup: dir.join(OPENMW_BACKUP_CFG),
        nerevar: dir.join(OPENMW_NEREVAR_CFG),
        dir,
    })
}

pub fn resolve_global_openmw_cfg_path() -> Option<PathBuf> {
    resolve_openmw_global_paths()
        .ok()
        .map(|paths| paths.active)
}

pub fn validate_nerevar_openmw_scaffold() -> Result<bool, String> {
    let paths = resolve_openmw_global_paths()?;
    if !paths.nerevar.is_file() {
        return Ok(false);
    }
    let contents = fs::read_to_string(&paths.nerevar)
        .map_err(|e| format!("Failed to read {}: {e}", paths.nerevar.display()))?;
    Ok(contents.contains("data="))
}

pub fn setup_nerevar_openmw_scaffold(morrowind_data_files: &Path) -> Result<(), String> {
    let paths = resolve_openmw_global_paths()?;
    fs::create_dir_all(&paths.dir)
        .map_err(|e| format!("Failed to create {}: {e}", paths.dir.display()))?;

    if !morrowind_data_files.join("Morrowind.esm").is_file() {
        return Err(format!(
            "Morrowind.esm not found in {}",
            morrowind_data_files.display()
        ));
    }

    if paths.active.is_file() {
        fs::copy(&paths.active, &paths.backup).map_err(|e| {
            format!(
                "Failed to back up {} to {}: {e}",
                paths.active.display(),
                paths.backup.display()
            )
        })?;
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

    apply_morrowind_ini_import(
        &ini,
        &paths.nerevar,
        seed,
        ImportOptions {
            encoding: IniEncoding::Win1252,
            import_game_files: false,
            import_archives: true,
        },
        morrowind_data_files,
    )
}

pub fn begin_global_openmw_launch(launch_cfg_path: &Path) -> Result<GlobalOpenMwLaunchSession, String> {
    let paths = resolve_openmw_global_paths()?;
    if !paths.nerevar.is_file() {
        return Err(
            "Nerevar OpenMW scaffold is missing (openmw.nerevar.cfg). Complete onboarding first."
                .to_string(),
        );
    }
    if !launch_cfg_path.is_file() {
        return Err(format!(
            "Instance launch config not found at {}",
            launch_cfg_path.display()
        ));
    }

    let had_active_cfg = paths.active.is_file();
    let had_backup = paths.backup.is_file();
    let restore_strategy = resolve_restore_strategy(had_active_cfg, had_backup);

    if had_active_cfg {
        fs::copy(&paths.active, &paths.backup).map_err(|e| {
            format!(
                "Failed to back up {} to {}: {e}",
                paths.active.display(),
                paths.backup.display()
            )
        })?;
    }

    let nerevar = fs::read_to_string(&paths.nerevar)
        .map_err(|e| format!("Failed to read {}: {e}", paths.nerevar.display()))?;
    let launch = fs::read_to_string(launch_cfg_path)
        .map_err(|e| format!("Failed to read {}: {e}", launch_cfg_path.display()))?;
    let composed = compose_active_openmw_cfg(&nerevar, &launch);

    fs::write(&paths.active, composed).map_err(|e| {
        format!(
            "Failed to write active OpenMW config at {}: {e}",
            paths.active.display()
        )
    })?;

    Ok(GlobalOpenMwLaunchSession {
        paths,
        restore_strategy,
    })
}

pub fn restore_global_openmw_launch(session: GlobalOpenMwLaunchSession) -> Result<(), String> {
    apply_openmw_restore(&session.paths, session.restore_strategy)
}

fn resolve_restore_strategy(had_active_cfg: bool, had_backup: bool) -> OpenMwRestoreStrategy {
    if had_active_cfg || had_backup {
        OpenMwRestoreStrategy::FromBackup
    } else {
        OpenMwRestoreStrategy::RemoveActive
    }
}

fn apply_openmw_restore(
    paths: &OpenMwGlobalPaths,
    strategy: OpenMwRestoreStrategy,
) -> Result<(), String> {
    match strategy {
        OpenMwRestoreStrategy::FromBackup => {
            if paths.backup.is_file() {
                fs::copy(&paths.backup, &paths.active).map_err(|e| {
                    format!(
                        "Failed to restore {} from {}: {e}",
                        paths.active.display(),
                        paths.backup.display()
                    )
                })?;
                return Ok(());
            }

            if paths.active.is_file() {
                fs::remove_file(&paths.active).map_err(|e| {
                    format!(
                        "Failed to remove swapped OpenMW config at {}: {e}",
                        paths.active.display()
                    )
                })?;
            }
        }
        OpenMwRestoreStrategy::RemoveActive => {
            if paths.active.is_file() {
                fs::remove_file(&paths.active).map_err(|e| {
                    format!(
                        "Failed to remove swapped OpenMW config at {}: {e}",
                        paths.active.display()
                    )
                })?;
            }
        }
    }

    Ok(())
}

pub fn read_nerevar_base_data_path() -> Option<String> {
    let paths = resolve_openmw_global_paths().ok()?;
    if !paths.nerevar.is_file() {
        return None;
    }
    let cfg = load_cfg_file(&paths.nerevar).ok()?;
    cfg.get("data").and_then(|values| values.first().cloned())
}

pub fn read_first_global_data_path() -> Option<String> {
    read_nerevar_base_data_path().or_else(|| {
        let path = resolve_global_openmw_cfg_path()?;
        if !path.exists() {
            return None;
        }
        let cfg = load_cfg_file(&path).ok()?;
        cfg.get("data").and_then(|values| values.first().cloned())
    })
}

fn compose_active_openmw_cfg(nerevar_base: &str, launch_overlay: &str) -> String {
    let base = parse_cfg_contents(nerevar_base);
    let launch = parse_cfg_contents(launch_overlay);
    let merged = merge_launch_overlay(base, launch);

    let mut out = String::from("# Nerevar active OpenMW config (managed during TES3MP launch)\n");
    out.push_str(&cfg_to_string(&merged));
    out
}

fn merge_launch_overlay(mut base: MultiStrMap, launch: MultiStrMap) -> MultiStrMap {
    if let Some(encoding) = launch.get("encoding").and_then(|values| values.first()) {
        base.insert("encoding".to_string(), vec![encoding.clone()]);
    }

    if let Some(content) = launch.get("content") {
        base.insert("content".to_string(), content.clone());
    }

    if let Some(launch_data) = launch.get("data") {
        let base_data = base.entry("data".to_string()).or_default();
        for path in launch_data {
            if !base_data
                .iter()
                .any(|existing| data_paths_equal(existing, path))
            {
                base_data.push(path.clone());
            }
        }
    }

    base
}

fn normalize_data_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .replace('/', "\\")
        .to_lowercase()
}

fn data_paths_equal(left: &str, right: &str) -> bool {
    normalize_data_path(left) == normalize_data_path(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_merges_launch_overlay_without_duplicate_keys() {
        let composed = compose_active_openmw_cfg(
            "encoding=win1252\ndata=\"C:\\\\Morrowind\\\\Data Files\"",
            "encoding=win1252\ndata=\"C:\\\\Morrowind\\\\Data Files\"\ndata=\"C:\\\\mods\\\\Better Bodies\"\ncontent=Morrowind.esm\ncontent=Tribunal.esm\ncontent=Bloodmoon.esm\ncontent=Better Bodies.esp",
        );
        assert_eq!(composed.matches("encoding=").count(), 1);
        assert_eq!(composed.matches("data=").count(), 2);
        assert!(composed.contains("content=Better Bodies.esp"));
    }

    #[test]
    fn data_paths_equal_ignores_quotes_and_case() {
        assert!(data_paths_equal(
            r#""C:\Morrowind\Data Files""#,
            "c:/morrowind/data files"
        ));
    }

    #[test]
    fn restore_strategy_prefers_backup_when_available() {
        assert_eq!(
            resolve_restore_strategy(false, true),
            OpenMwRestoreStrategy::FromBackup
        );
        assert_eq!(
            resolve_restore_strategy(true, false),
            OpenMwRestoreStrategy::FromBackup
        );
        assert_eq!(
            resolve_restore_strategy(false, false),
            OpenMwRestoreStrategy::RemoveActive
        );
    }

    #[test]
    fn restore_removes_composed_cfg_when_no_prior_config() {
        let dir = std::env::temp_dir().join(format!("nerevar-openmw-restore-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let paths = OpenMwGlobalPaths {
            dir: dir.clone(),
            active: dir.join(OPENMW_CFG),
            backup: dir.join(OPENMW_BACKUP_CFG),
            nerevar: dir.join(OPENMW_NEREVAR_CFG),
        };
        fs::write(&paths.active, "content=Better Bodies.esp\n").unwrap();

        apply_openmw_restore(&paths, OpenMwRestoreStrategy::RemoveActive).unwrap();
        assert!(!paths.active.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_copies_backup_when_present() {
        let dir =
            std::env::temp_dir().join(format!("nerevar-openmw-restore-backup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let paths = OpenMwGlobalPaths {
            dir: dir.clone(),
            active: dir.join(OPENMW_CFG),
            backup: dir.join(OPENMW_BACKUP_CFG),
            nerevar: dir.join(OPENMW_NEREVAR_CFG),
        };
        fs::write(&paths.backup, "encoding=win1252\n").unwrap();
        fs::write(&paths.active, "content=Better Bodies.esp\n").unwrap();

        apply_openmw_restore(&paths, OpenMwRestoreStrategy::FromBackup).unwrap();
        assert_eq!(
            fs::read_to_string(&paths.active).unwrap(),
            "encoding=win1252\n"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
