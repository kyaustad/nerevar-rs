//! The file actions of Modding-OpenMW downloads, with umo's meaning. Paths are relative to the category folder and
//! may not leave it.

use std::path::{Component, Path, PathBuf};

use super::{CopyRenameFile, ModAction};

fn resolve(base: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.is_empty() || !path.components().all(|part| matches!(part, Component::Normal(_) | Component::CurDir)) {
        return Err(format!("Unallowed path in a mod action: {relative}"));
    }
    Ok(base.join(path))
}

fn copy_any(src: &Path, dst: &Path, force: bool) -> Result<(), String> {
    if src.is_dir() {
        if dst.exists() && !force {
            return Err(format!("{} exists already", dst.display()));
        }
        copy_dir(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // Like shutil.copy, a folder destination receives the file
        let target = if dst.is_dir() { dst.join(src.file_name().unwrap_or_default()) } else { dst.to_path_buf() };
        std::fs::copy(src, &target)
            .map(|_| ())
            .map_err(|e| format!("Failed to copy {} to {}: {e}", src.display(), target.display()))
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("Failed to read {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn remove(path: &Path) -> Result<(), String> {
    let result = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    };
    result.map_err(|e| format!("Failed to remove {}: {e}", path.display()))
}

/// Simple glob for file names: `*` and `?`.
fn glob_match(pattern: &str, name: &str) -> bool {
    fn go(p: &[char], n: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some('*'), _) => go(&p[1..], n) || (!n.is_empty() && go(p, &n[1..])),
            (Some('?'), Some(_)) => go(&p[1..], &n[1..]),
            (Some(a), Some(b)) => a == b && go(&p[1..], &n[1..]),
            _ => false,
        }
    }
    go(&pattern.chars().collect::<Vec<_>>(), &name.chars().collect::<Vec<_>>())
}

fn copy_rename_sources(base: &Path, file: &CopyRenameFile) -> Result<Vec<String>, String> {
    let source = Path::new(&file.src);
    let name = source.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if file.name.is_some() || !name.contains(['*', '?']) {
        return Ok(vec![file.src.clone()]);
    }
    let parent = source.parent().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
    let dir = resolve(base, if parent.is_empty() { "." } else { &parent })?;
    let mut matches: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read {}: {e}", dir.display()))?
        .flatten()
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|candidate| glob_match(&name, candidate))
        .map(|candidate| if parent.is_empty() { candidate } else { format!("{parent}/{candidate}") })
        .collect();
    matches.sort();
    if matches.is_empty() {
        return Err(format!("No files match {}", file.src));
    }
    Ok(matches)
}

/// Runs an action in the category folder `base`. Returns a note for actions Nerevar skips.
pub fn run(base: &Path, action: &ModAction) -> Result<Option<String>, String> {
    match action {
        ModAction::Remove { path, paths } => {
            for relative in path.iter().chain(paths.iter().flatten()) {
                remove(&resolve(base, relative)?)?;
            }
        }
        ModAction::Rename { src, dst } => {
            let (src, dst) = (resolve(base, src)?, resolve(base, dst)?);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::rename(&src, &dst)
                .map_err(|e| format!("Failed to rename {} to {}: {e}", src.display(), dst.display()))?;
        }
        ModAction::Copy { src, srcs, dst, force } => {
            let force = force.unwrap_or(false);
            if let Some(src) = src {
                copy_any(&resolve(base, src)?, &resolve(base, dst)?, force)?;
            }
            for src in srcs.iter().flatten() {
                let name = Path::new(src).file_name().ok_or_else(|| format!("Invalid copy source {src}"))?;
                copy_any(&resolve(base, src)?, &resolve(base, dst)?.join(name), force)?;
            }
        }
        ModAction::CopyRename { files, strip_prefix, dst, force } => {
            let force = force.unwrap_or(false);
            for file in files {
                for source in copy_rename_sources(base, file)? {
                    let original = Path::new(&source).file_name().unwrap_or_default().to_string_lossy().into_owned();
                    let name = match (&file.name, strip_prefix) {
                        (Some(name), _) => name.clone(),
                        (None, Some(prefix)) => original
                            .strip_prefix(prefix.as_str())
                            .ok_or_else(|| format!("{source} doesn't start with {prefix}"))?
                            .to_string(),
                        (None, None) => return Err(format!("No new name for {source}")),
                    };
                    if name.contains(['/', '\\', ':']) || name.is_empty() {
                        return Err(format!("Invalid file name {name}"));
                    }
                    copy_any(&resolve(base, &source)?, &resolve(base, dst)?.join(name), force)?;
                }
            }
        }
        // Run after every mod is installed, with the MOMW Tools Pack's tes3cmd (tools::clean)
        ModAction::Clean { .. } => {}
        ModAction::Unknown => return Ok(Some("skipped an action this version of Nerevar doesn't know".into())),
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_umo_actions_inside_the_category_folder() {
        let base = std::env::temp_dir().join(format!("nerevar-actions-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(base.join("Mod/Sub")).unwrap();
        std::fs::write(base.join("Mod/Sub/a.esp"), "a").unwrap();
        std::fs::write(base.join("Mod/Sub/prefix_b.dds"), "b").unwrap();
        std::fs::write(base.join("Mod/junk.txt"), "j").unwrap();

        run(&base, &ModAction::Copy { src: Some("Mod/Sub".into()), srcs: None, dst: "Mod/Copy".into(), force: None })
            .unwrap();
        assert!(base.join("Mod/Copy/a.esp").is_file());
        run(&base, &ModAction::CopyRename {
            files: vec![CopyRenameFile { src: "Mod/Sub/prefix_*.dds".into(), name: None }],
            strip_prefix: Some("prefix_".into()),
            dst: "Mod/Renamed".into(),
            force: None,
        })
        .unwrap();
        assert!(base.join("Mod/Renamed/b.dds").is_file());
        run(&base, &ModAction::Remove { path: Some("Mod/junk.txt".into()), paths: None }).unwrap();
        assert!(!base.join("Mod/junk.txt").exists());
        assert!(run(&base, &ModAction::Remove { path: Some("../outside".into()), paths: None }).is_err());
        assert!(run(&base, &ModAction::Clean { path: "Mod/Sub/a.esp".into(), arguments: None }).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&base);
    }
}
