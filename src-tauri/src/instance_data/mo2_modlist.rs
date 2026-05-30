use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::types::{LoadOrder, LoadOrderEntry, PluginEntry};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Mo2ModlistImportReport {
    pub matched_mods: u32,
    pub disabled_mods: u32,
    pub missing_mod_directories: Vec<String>,
    pub missing_plugins: Vec<String>,
    pub imported_plugins: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Mo2ModlistImportResult {
    pub load_order: LoadOrder,
    pub report: Mo2ModlistImportReport,
}

#[derive(Debug, Clone)]
struct Mo2ModImport {
    directory: String,
    priority: u32,
    plugins: Vec<Mo2PluginImport>,
}

#[derive(Debug, Clone)]
struct Mo2PluginImport {
    name: String,
    enabled: bool,
    load_order: Option<u32>,
}

#[derive(Debug, Clone)]
struct Mo2ModlistImport {
    mods: HashMap<String, Mo2ModImport>,
}

pub fn import_mo2_modlist_from_csv(
    load_order: &LoadOrder,
    contents: &str,
) -> Result<(LoadOrder, Mo2ModlistImportReport), String> {
    let import = parse_mo2_modlist_csv(contents)?;
    Ok(apply_mo2_modlist(load_order, &import))
}

fn parse_mo2_modlist_csv(contents: &str) -> Result<Mo2ModlistImport, String> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(contents.as_bytes());

    let headers = reader
        .headers()
        .map_err(|error| format!("Invalid CSV header: {error}"))?
        .clone();

    let directory_idx = find_column(&headers, &["mod directory", "mod_directory"])?;
    let priority_idx = find_column(&headers, &["mod priority", "mod_priority"])?;
    let plugin_idx = find_column(&headers, &["plugin name", "plugin_name"])?;
    let enabled_idx = find_column(&headers, &["plugin enabled", "plugin_enabled"])?;
    let load_order_idx = find_column(
        &headers,
        &["plugin load order", "plugin_load_order"],
    )?;

    let mut mods: HashMap<String, Mo2ModImport> = HashMap::new();

    for row in reader.records() {
        let row = row.map_err(|error| format!("Invalid CSV row: {error}"))?;
        let directory = field(&row, directory_idx)?;
        if directory.is_empty() {
            continue;
        }

        let priority = field(&row, priority_idx)?
            .parse::<u32>()
            .map_err(|_| format!("Invalid mod priority for {directory}"))?;

        let key = normalize_key(&directory);
        let entry = mods.entry(key).or_insert_with(|| Mo2ModImport {
            directory: directory.clone(),
            priority,
            plugins: Vec::new(),
        });

        if entry.priority != priority {
            entry.priority = entry.priority.min(priority);
        }

        let plugin_name = field(&row, plugin_idx)?;
        if plugin_name.is_empty() {
            continue;
        }

        entry.plugins.push(Mo2PluginImport {
            name: plugin_name,
            enabled: parse_bool(&field(&row, enabled_idx)?),
            load_order: parse_optional_u32(&field(&row, load_order_idx)?),
        });
    }

    if mods.is_empty() {
        return Err("CSV contains no mod rows".into());
    }

    Ok(Mo2ModlistImport { mods })
}

fn apply_mo2_modlist(
    load_order: &LoadOrder,
    import: &Mo2ModlistImport,
) -> (LoadOrder, Mo2ModlistImportReport) {
    let mut report = Mo2ModlistImportReport {
        matched_mods: 0,
        disabled_mods: 0,
        missing_mod_directories: Vec::new(),
        missing_plugins: Vec::new(),
        imported_plugins: 0,
    };

    let mut matched_keys = HashSet::new();
    let mut entries: Vec<LoadOrderEntry> = load_order.entries.clone();

    for entry in &mut entries {
        let key = normalize_key(&entry.relative_dir);
        let Some(mod_import) = import.mods.get(&key) else {
            if entry.enabled {
                report.disabled_mods += 1;
            }
            entry.enabled = false;
            for plugin in &mut entry.plugins {
                plugin.enabled = false;
            }
            continue;
        };

        matched_keys.insert(key);
        report.matched_mods += 1;
        entry.enabled = true;
        entry.priority = mod_import.priority.saturating_mul(10);

        let csv_plugins: HashMap<String, &Mo2PluginImport> = mod_import
            .plugins
            .iter()
            .map(|plugin| (normalize_key(&plugin.name), plugin))
            .collect();

        for plugin in &mut entry.plugins {
            if let Some(csv_plugin) = csv_plugins.get(&normalize_key(&plugin.file)) {
                plugin.enabled = csv_plugin.enabled;
                if csv_plugin.enabled {
                    report.imported_plugins += 1;
                }
            } else {
                plugin.enabled = false;
            }
        }

        for csv_plugin in &mod_import.plugins {
            if entry
                .plugins
                .iter()
                .any(|plugin| plugin.file.eq_ignore_ascii_case(&csv_plugin.name))
            {
                continue;
            }
            report
                .missing_plugins
                .push(format!("{}/{}", entry.relative_dir, csv_plugin.name));
        }

        entry.plugins.sort_by(|left, right| {
            let left_order = csv_plugins
                .get(&normalize_key(&left.file))
                .and_then(|plugin| plugin.load_order);
            let right_order = csv_plugins
                .get(&normalize_key(&right.file))
                .and_then(|plugin| plugin.load_order);
            left_order
                .unwrap_or(u32::MAX)
                .cmp(&right_order.unwrap_or(u32::MAX))
                .then_with(|| left.file.to_ascii_lowercase().cmp(&right.file.to_ascii_lowercase()))
        });
    }

    for mod_import in import.mods.values() {
        let key = normalize_key(&mod_import.directory);
        if matched_keys.contains(&key) {
            continue;
        }
        report
            .missing_mod_directories
            .push(mod_import.directory.clone());
    }

    report.missing_mod_directories.sort();
    report.missing_plugins.sort();

    let content_order = build_content_order(import);

    (
        LoadOrder {
            version: load_order.version,
            base_game_data: load_order.base_game_data.clone(),
            content_order: Some(content_order),
            entries,
        },
        report,
    )
}

fn build_content_order(import: &Mo2ModlistImport) -> Vec<String> {
    let mut rows: Vec<(u32, String)> = Vec::new();

    for mod_import in import.mods.values() {
        for plugin in &mod_import.plugins {
            if !plugin.enabled {
                continue;
            }
            rows.push((
                plugin.load_order.unwrap_or(u32::MAX),
                plugin.name.clone(),
            ));
        }
    }

    rows.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.to_ascii_lowercase().cmp(&right.1.to_ascii_lowercase()))
    });

    let mut seen = HashSet::new();
    rows.into_iter()
        .filter_map(|(_, name)| {
            let key = normalize_key(&name);
            if seen.insert(key) {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn find_column(headers: &csv::StringRecord, aliases: &[&str]) -> Result<usize, String> {
    for (index, header) in headers.iter().enumerate() {
        let normalized = header.trim().to_ascii_lowercase();
        if aliases
            .iter()
            .any(|alias| normalized == *alias)
        {
            return Ok(index);
        }
    }
    Err(format!(
        "Missing CSV column (expected one of: {})",
        aliases.join(", ")
    ))
}

fn field(row: &csv::StringRecord, index: usize) -> Result<String, String> {
    row.get(index)
        .map(str::trim)
        .map(str::to_string)
        .ok_or_else(|| "CSV row is missing expected columns".into())
}

fn normalize_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes"
    )
}

fn parse_optional_u32(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_data::types::{LoadOrderEntry, PackageKind, LOAD_ORDER_VERSION};

    const SAMPLE_CSV: &str = r#"Mod Directory,Mod Priority,Plugin Name,Plugin Enabled,Plugin Load Order
Tamriel Rebuilt,4,TR_Factions.esp,True,64
Tamriel Rebuilt,4,TR_Mainland.esm,True,7
Skyrim Home Of The Nords,5,Sky_Main.esm,True,16
Beautiful Cities of Morrowind,8,BCOM - Open Vivec Arena_Skywind.esp,True,215
Beautiful Cities of Morrowind,8,Beautiful cities of Morrowind.ESP,True,177
Replacer Only,9,,, 
"#;

    #[test]
    fn parses_mo2_csv_rows() {
        let import = parse_mo2_modlist_csv(SAMPLE_CSV).unwrap();
        assert_eq!(import.mods.len(), 4);
        assert_eq!(
            import.mods.get(&normalize_key("Tamriel Rebuilt")).unwrap().plugins.len(),
            2
        );
    }

    #[test]
    fn apply_matches_directories_and_builds_content_order() {
        let import = parse_mo2_modlist_csv(SAMPLE_CSV).unwrap();
        let load_order = LoadOrder {
            version: LOAD_ORDER_VERSION,
            base_game_data: None,
            content_order: None,
            entries: vec![
                LoadOrderEntry {
                    id: "1".into(),
                    name: "Tamriel Rebuilt".into(),
                    kind: PackageKind::Mod,
                    relative_dir: "Tamriel Rebuilt".into(),
                    enabled: false,
                    priority: 999,
                    plugins: vec![
                        PluginEntry {
                            file: "TR_Mainland.esm".into(),
                            enabled: false,
                        },
                        PluginEntry {
                            file: "TR_Factions.esp".into(),
                            enabled: false,
                        },
                    ],
                    tree_checksum: None,
                },
                LoadOrderEntry {
                    id: "2".into(),
                    name: "Unused Mod".into(),
                    kind: PackageKind::Mod,
                    relative_dir: "Unused Mod".into(),
                    enabled: true,
                    priority: 10,
                    plugins: vec![],
                    tree_checksum: None,
                },
            ],
        };

        let (updated, report) = apply_mo2_modlist(&load_order, &import);
        assert_eq!(report.matched_mods, 1);
        assert_eq!(report.disabled_mods, 1);
        assert!(report
            .missing_mod_directories
            .iter()
            .any(|dir| dir.contains("Skyrim")));

        let tr = updated
            .entries
            .iter()
            .find(|entry| entry.relative_dir == "Tamriel Rebuilt")
            .unwrap();
        assert!(tr.enabled);
        assert_eq!(tr.priority, 40);
        assert!(tr.plugins[0].file.eq_ignore_ascii_case("TR_Mainland.esm"));
        assert!(tr.plugins[0].enabled);
        assert!(tr.plugins[1].file.eq_ignore_ascii_case("TR_Factions.esp"));

        let content_order = updated.content_order.expect("content order");
        let mainland_pos = content_order
            .iter()
            .position(|name| name.eq_ignore_ascii_case("TR_Mainland.esm"))
            .unwrap();
        let factions_pos = content_order
            .iter()
            .position(|name| name.eq_ignore_ascii_case("TR_Factions.esp"))
            .unwrap();
        assert!(mainland_pos < factions_pos);
    }
}
