use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::paths::package_abs_path;
use super::types::{LoadOrder, NerevarManifest, ResolvedOpenMwConfig};
use crate::openmw_ini_importer::{quote_data_path, sort_content_plugins, PluginIndex};

const DEFAULT_BASE_ESMS: &[&str] = &["Morrowind.esm", "Tribunal.esm", "Bloodmoon.esm"];

pub fn resolve_load_order(data_dir: &Path, load_order: &LoadOrder) -> Result<ResolvedOpenMwConfig, String> {
    let mut data_paths = Vec::new();

    if let Some(base) = &load_order.base_game_data {
        if !base.is_empty() {
            data_paths.push(PathBuf::from(strip_quotes(base)));
        }
    }

    let mut enabled_entries: Vec<_> = load_order
        .entries
        .iter()
        .filter(|e| e.enabled)
        .collect();
    enabled_entries.sort_by_key(|e| e.priority);

    for entry in &enabled_entries {
        data_paths.push(package_abs_path(data_dir, &entry.relative_dir));
    }

    let mut plugin_names = Vec::new();
    for entry in enabled_entries {
        for plugin in &entry.plugins {
            if plugin.enabled {
                plugin_names.push(plugin.file.clone());
            }
        }
    }

    let index = PluginIndex::build(&data_paths);
    let mut content = resolve_content_plugins(load_order, &index, &plugin_names)?;

    if let Some(base_path) = load_order
        .base_game_data
        .as_ref()
        .map(|s| PathBuf::from(strip_quotes(s)))
    {
        for esm in DEFAULT_BASE_ESMS {
            if base_path.join(esm).exists()
                && !content.iter().any(|c| c.eq_ignore_ascii_case(esm))
            {
                content.insert(0, esm.to_string());
            }
        }
        content = reorder_base_esms(content);
    }

    let data_paths_quoted: Vec<String> = data_paths.iter().map(|p| quote_data_path(p)).collect();

    Ok(ResolvedOpenMwConfig {
        encoding: "win1252".to_string(),
        data_paths: data_paths_quoted,
        content,
    })
}

/// Build launch config for a synced client using local data paths and the host's plugin order.
pub fn resolve_synced_load_order(
    data_dir: &Path,
    load_order: &LoadOrder,
    manifest: &NerevarManifest,
) -> Result<ResolvedOpenMwConfig, String> {
    let local = resolve_load_order(data_dir, load_order)?;
    let data_paths: Vec<PathBuf> = local
        .data_paths
        .iter()
        .map(|p| PathBuf::from(strip_quotes(p)))
        .collect();

    let index = PluginIndex::build(&data_paths);
    let mut content = Vec::new();
    for plugin in &manifest.resolved.content {
        if index.find(plugin).is_some() {
            content.push(plugin.clone());
        }
    }

    if content.is_empty() {
        content = local.content;
    }

    Ok(ResolvedOpenMwConfig {
        encoding: local.encoding,
        data_paths: local.data_paths,
        content,
    })
}

fn resolve_content_plugins(
    load_order: &LoadOrder,
    index: &PluginIndex,
    enabled_plugin_names: &[String],
) -> Result<Vec<String>, String> {
    if let Some(content_order) = &load_order.content_order {
        return Ok(resolve_explicit_content_order(
            index,
            content_order,
            enabled_plugin_names,
        ));
    }

    sort_content_plugins(index, enabled_plugin_names)
}

fn resolve_explicit_content_order(
    index: &PluginIndex,
    content_order: &[String],
    enabled_plugin_names: &[String],
) -> Vec<String> {
    let enabled: HashSet<String> = enabled_plugin_names
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();

    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for name in content_order {
        let key = name.to_ascii_lowercase();
        if !enabled.contains(&key) {
            continue;
        }
        if index.find(name).is_none() {
            continue;
        }
        if seen.insert(key) {
            result.push(name.clone());
        }
    }

    for name in enabled_plugin_names {
        let key = name.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        if index.find(name).is_some() {
            result.push(name.clone());
        }
    }

    result
}

fn strip_quotes(path: &str) -> String {
    let trimmed = path.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(trimmed)
        .to_string()
}

fn reorder_base_esms(mut content: Vec<String>) -> Vec<String> {
    let mut ordered = Vec::new();
    for base in DEFAULT_BASE_ESMS {
        if let Some(pos) = content.iter().position(|c| c.eq_ignore_ascii_case(base)) {
            ordered.push(content.remove(pos));
        }
    }
    ordered.extend(content);
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::paths::ensure_instance_data_layout;
    use crate::instance_data::types::{
        LoadOrder, LoadOrderEntry, NerevarManifest, PackageKind, ResolvedOpenMwConfig,
        LOAD_ORDER_VERSION,
    };
    use crate::instance_settings::InstanceSettings;

    #[test]
    fn resolve_orders_data_paths_by_priority() {
        let dir = std::env::temp_dir().join(format!("nerevar-resolve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ensure_instance_data_layout(&dir).unwrap();

        let low = dir.join("low");
        let high = dir.join("high");
        std::fs::create_dir_all(&low).unwrap();
        std::fs::create_dir_all(&high).unwrap();

        let load_order = LoadOrder {
            version: LOAD_ORDER_VERSION,
            base_game_data: None,
            content_order: None,
            entries: vec![
                LoadOrderEntry {
                    id: "1".into(),
                    name: "low".into(),
                    kind: PackageKind::Replacer,
                    relative_dir: "low".into(),
                    enabled: true,
                    priority: 10,
                    plugins: vec![],
                    tree_checksum: None,
                },
                LoadOrderEntry {
                    id: "2".into(),
                    name: "high".into(),
                    kind: PackageKind::Replacer,
                    relative_dir: "high".into(),
                    enabled: true,
                    priority: 20,
                    plugins: vec![],
                    tree_checksum: None,
                },
            ],
        };

        let resolved = resolve_load_order(&dir, &load_order).unwrap();
        assert_eq!(resolved.data_paths.len(), 2);
        assert!(resolved.data_paths[1].contains("high"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_synced_uses_host_content_when_local_plugins_empty() {
        let dir = std::env::temp_dir().join(format!("nerevar-synced-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        ensure_instance_data_layout(&dir).unwrap();

        let pkg = dir.join("Better Bodies");
        std::fs::create_dir_all(pkg.join("nested")).unwrap();
        std::fs::write(pkg.join("nested/Better Bodies.esp"), b"").unwrap();

        let load_order = LoadOrder {
            version: LOAD_ORDER_VERSION,
            base_game_data: None,
            content_order: None,
            entries: vec![LoadOrderEntry {
                id: "1".into(),
                name: "Better Bodies".into(),
                kind: PackageKind::Mod,
                relative_dir: "Better Bodies".into(),
                enabled: true,
                priority: 10,
                plugins: vec![],
                tree_checksum: None,
            }],
        };

        let manifest = NerevarManifest {
            version: 1,
            instance_id: "x".into(),
            instance_name: "x".into(),
            generated_at: String::new(),
            base_game_data: None,
            packages: vec![],
            resolved: ResolvedOpenMwConfig {
                encoding: "win1252".into(),
                data_paths: vec![],
                content: vec!["Better Bodies.esp".into()],
            },
            total_download_bytes: 0,
            tes3mp_server_port: 25565,
            tes3mp_server_password: String::new(),
            required_data_files: vec![],
            instance_settings: InstanceSettings::default(),
        };

        let resolved = resolve_synced_load_order(&dir, &load_order, &manifest).unwrap();
        assert!(
            resolved
                .content
                .iter()
                .any(|p| p.eq_ignore_ascii_case("Better Bodies.esp")),
            "expected Better Bodies.esp in synced content, got {:?}",
            resolved.content
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
