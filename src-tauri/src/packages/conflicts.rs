//! Files that packages overwrite in each other. OpenMW's VFS takes a file from the last data folder that has it
//! (ignoring case), so, as in Mod Organizer 2, a package "wins" the files it shares with packages loaded before it.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

use crate::instance_data::{find_instance_by_id, load_load_order, package_abs_path, resolve_package_data_dir, LoadOrder};
use crate::AppState;

const EXAMPLES: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConflictWith {
    pub entry_id: String,
    pub name: String,
    pub files: u32,
    /// Some of the files, relative to the packages
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PackageConflicts {
    pub entry_id: String,
    /// Packages loaded before this one whose files this one replaces
    pub overwrites: Vec<ConflictWith>,
    /// Packages loaded after this one that replace its files
    pub overwritten_by: Vec<ConflictWith>,
    /// Files of this package that no other package replaces
    pub winning_files: u32,
    /// Files of this package that a later package replaces
    pub losing_files: u32,
}

fn list_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else { continue };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(dir) {
                files.push(relative.to_string_lossy().replace('\\', "/").to_ascii_lowercase());
            }
        }
    }
    files
}

/// Conflicts between the enabled packages, in load order.
pub fn find_conflicts(data_dir: &Path, load_order: &LoadOrder) -> Vec<PackageConflicts> {
    let mut entries: Vec<_> = load_order.entries.iter().filter(|e| e.enabled && !e.is_core_shaders()).collect();
    entries.sort_by_key(|e| (e.shader_kind.is_some(), e.priority));

    let files: Vec<Vec<String>> =
        entries.par_iter().map(|entry| list_files(&package_abs_path(data_dir, &entry.relative_dir))).collect();

    // file -> index of the package OpenMW takes it from
    let mut winner: HashMap<&str, usize> = HashMap::new();
    for (index, package_files) in files.iter().enumerate() {
        for file in package_files {
            winner.insert(file.as_str(), index);
        }
    }

    let mut result: Vec<PackageConflicts> = entries
        .iter()
        .map(|e| PackageConflicts {
            entry_id: e.id.clone(),
            overwrites: Vec::new(),
            overwritten_by: Vec::new(),
            winning_files: 0,
            losing_files: 0,
        })
        .collect();
    for (index, package_files) in files.iter().enumerate() {
        for file in package_files {
            let wins = winner[file.as_str()];
            if wins == index {
                result[index].winning_files += 1;
            } else {
                result[index].losing_files += 1;
            }
        }
    }

    // Every pair of packages sharing files, also when a third, later package replaces them again (MO2 shows those too)
    let mut shared: HashMap<(usize, usize), Vec<&str>> = HashMap::new();
    let mut owners: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, package_files) in files.iter().enumerate() {
        for file in package_files {
            owners.entry(file.as_str()).or_default().push(index);
        }
    }
    for (file, indices) in &owners {
        for (position, &later) in indices.iter().enumerate().skip(1) {
            for &earlier in &indices[..position] {
                shared.entry((earlier, later)).or_default().push(file);
            }
        }
    }

    for ((earlier, later), mut files) in shared {
        files.sort();
        let count = files.len() as u32;
        let examples: Vec<String> = files.iter().take(EXAMPLES).map(|f| f.to_string()).collect();
        result[later].overwrites.push(ConflictWith {
            entry_id: entries[earlier].id.clone(),
            name: entries[earlier].name.clone(),
            files: count,
            examples: examples.clone(),
        });
        result[earlier].overwritten_by.push(ConflictWith {
            entry_id: entries[later].id.clone(),
            name: entries[later].name.clone(),
            files: count,
            examples,
        });
    }
    let position: HashMap<&str, usize> = entries.iter().enumerate().map(|(i, e)| (e.id.as_str(), i)).collect();
    for package in &mut result {
        package.overwrites.sort_by_key(|c| position[c.entry_id.as_str()]);
        package.overwritten_by.sort_by_key(|c| position[c.entry_id.as_str()]);
    }
    result.retain(|p| !p.overwrites.is_empty() || !p.overwritten_by.is_empty());
    result
}

#[tauri::command]
/// Conflicts of the instance's packages, in `load_order` (the one being edited) or the saved load order.
pub async fn package_conflicts(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    load_order: Option<LoadOrder>,
) -> Result<Vec<PackageConflicts>, String> {
    let data_dir = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        resolve_package_data_dir(
            find_instance_by_id(&guard.nerevar_config, &instance_id)
                .ok_or_else(|| format!("Instance not found: {instance_id}"))?,
        )
    };
    tauri::async_runtime::spawn_blocking(move || {
        let load_order = match load_order {
            Some(load_order) => load_order,
            None => load_load_order(&data_dir)?,
        };
        Ok(find_conflicts(&data_dir, &load_order))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::{LoadOrderEntry, PackageKind};

    fn entry(id: &str, priority: u32) -> LoadOrderEntry {
        LoadOrderEntry {
            id: id.into(),
            name: id.into(),
            kind: PackageKind::Replacer,
            relative_dir: id.into(),
            enabled: true,
            priority,
            plugins: vec![],
            tree_checksum: None,
            shader_kind: None,
        }
    }

    #[test]
    fn later_packages_win_shared_files() {
        let data = std::env::temp_dir().join(format!("nerevar-conflicts-{}", uuid::Uuid::new_v4()));
        for (dir, file) in [("A", "textures/Rock.dds"), ("A", "meshes/a.nif"), ("B", "textures/rock.dds"), ("C", "textures/rock.dds"), ("C", "only.txt")] {
            let path = data.join(dir).join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let order = LoadOrder { entries: vec![entry("A", 10), entry("B", 20), entry("C", 30)], ..Default::default() };
        let conflicts = find_conflicts(&data, &order);
        let get = |id: &str| conflicts.iter().find(|c| c.entry_id == id).unwrap();
        assert_eq!(get("A").losing_files, 1);
        assert_eq!(get("A").winning_files, 1);
        assert_eq!(get("A").overwritten_by.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["B", "C"]);
        assert_eq!(get("C").overwrites.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);
        assert_eq!(get("C").losing_files, 0);
        assert_eq!(get("B").overwrites[0].examples, vec!["textures/rock.dds"]);
        let _ = std::fs::remove_dir_all(&data);
    }
}
