use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use encoding_rs::Encoding;

use super::content_files::{self, is_record_plugin};
use super::esm_header;
use super::fallback_keys::FALLBACK_INI_KEYS;
use super::plugin_index::PluginIndex;

pub type MultiStrMap = BTreeMap<String, Vec<String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IniEncoding {
    Win1250,
    Win1251,
    Win1252,
}

impl IniEncoding {
    pub fn from_name(name: &str) -> Self {
        match name {
            "win1250" => Self::Win1250,
            "win1251" => Self::Win1251,
            _ => Self::Win1252,
        }
    }

    fn encoding(self) -> &'static Encoding {
        match self {
            Self::Win1250 => encoding_rs::WINDOWS_1250,
            Self::Win1251 => encoding_rs::WINDOWS_1251,
            Self::Win1252 => encoding_rs::WINDOWS_1252,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub encoding: IniEncoding,
    pub import_game_files: bool,
    pub import_archives: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            encoding: IniEncoding::Win1252,
            import_game_files: true,
            import_archives: true,
        }
    }
}

/// Port of OpenMW `openmw-iniimporter` (see apps/mwiniimporter).
pub fn import_morrowind_ini(
    morrowind_ini: &Path,
    openmw_cfg: &Path,
    seed: MultiStrMap,
    options: ImportOptions,
) -> Result<(), String> {
    if !morrowind_ini.exists() {
        return Err(format!(
            "Morrowind.ini not found at {}",
            morrowind_ini.display()
        ));
    }

    let ini = load_ini_file(morrowind_ini, options.encoding)?;
    let mut cfg = if openmw_cfg.exists() {
        load_cfg_file(openmw_cfg)?
    } else {
        seed.clone()
    };

    for (key, values) in seed {
        if !values.is_empty() {
            cfg.insert(key, values);
        }
    }

    merge(&mut cfg, &ini);
    merge_fallback(&mut cfg, &ini);

    if options.import_game_files {
        import_game_files(&mut cfg, &ini, morrowind_ini, options.encoding)?;
    }

    if options.import_archives {
        import_archives(&mut cfg, &ini);
    }

    if let Some(parent) = openmw_cfg.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut file = fs::File::create(openmw_cfg).map_err(|e| e.to_string())?;
    write_to_file(&mut file, &cfg).map_err(|e| e.to_string())?;
    Ok(())
}

fn decode_line(bytes: &[u8], encoding: IniEncoding) -> String {
    let (cow, _, _) = encoding.encoding().decode(bytes);
    cow.into_owned().trim_end_matches('\r').to_string()
}

pub fn load_ini_file(path: &Path, encoding: IniEncoding) -> Result<MultiStrMap, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let mut map: MultiStrMap = BTreeMap::new();
    let mut section = String::new();

    for raw_line in bytes.split(|&b| b == b'\n') {
        let line = decode_line(raw_line, encoding);
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                if end >= 2 {
                    section = line[1..end].to_string();
                }
            }
            continue;
        }

        let line = line.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let Some((key_part, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        let key = if section.is_empty() {
            key_part.trim().to_string()
        } else {
            format!("{}:{}", section, key_part.trim())
        };

        map.entry(key).or_default().push(value.to_string());
    }

    Ok(map)
}

pub fn parse_cfg_contents(contents: &str) -> MultiStrMap {
    let mut map: MultiStrMap = BTreeMap::new();

    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        map.entry(key.trim().to_string())
            .or_default()
            .push(value.trim().to_string());
    }

    map
}

pub fn load_cfg_file(path: &Path) -> Result<MultiStrMap, String> {
    let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(parse_cfg_contents(&contents))
}

pub fn cfg_to_string(cfg: &MultiStrMap) -> String {
    let mut out = Vec::new();
    write_to_file(&mut out, cfg).expect("writing OpenMW cfg to memory should not fail");
    String::from_utf8(out).expect("OpenMW cfg should be valid UTF-8")
}

fn merge(cfg: &mut MultiStrMap, ini: &MultiStrMap) {
    const MERGE_MAP: &[(&str, &str)] = &[("no-sound", "General:Disable Audio")];

    for (cfg_key, ini_key) in MERGE_MAP {
        if let Some(values) = ini.get(*ini_key) {
            cfg.remove(*cfg_key);
            for value in values {
                insert_multistrmap(cfg, cfg_key, value);
            }
        }
    }
}

fn merge_fallback(cfg: &mut MultiStrMap, ini: &MultiStrMap) {
    cfg.remove("fallback");

    for ini_key in FALLBACK_INI_KEYS {
        let Some(values) = ini.get(*ini_key) else {
            continue;
        };
        for value in values {
            let mut fallback_key = ini_key.replace(' ', "_").replace(':', "_");
            fallback_key.push(',');
            fallback_key.push_str(value);
            insert_multistrmap(cfg, "fallback", &fallback_key);
        }
    }
}

fn import_archives(cfg: &mut MultiStrMap, ini: &MultiStrMap) {
    let mut archives = Vec::new();
    let base = "Archives:Archive ";

    for i in 0.. {
        let key = format!("{base}{i}");
        let Some(values) = ini.get(&key) else {
            break;
        };
        archives.extend(values.iter().cloned());
    }

    cfg.remove("fallback-archive");
    cfg.insert(
        "fallback-archive".to_string(),
        vec!["Morrowind.bsa".to_string()],
    );
    if let Some(entry) = cfg.get_mut("fallback-archive") {
        entry.extend(archives);
    }
}

fn import_game_files(
    cfg: &mut MultiStrMap,
    ini: &MultiStrMap,
    ini_path: &Path,
    encoding: IniEncoding,
) -> Result<(), String> {
    let mut content_files: Vec<(SystemTime, PathBuf)> = Vec::new();
    let base = "Game Files:GameFile";

    let mut data_paths: Vec<PathBuf> = Vec::new();
    if let Some(paths) = cfg.get("data") {
        add_paths(&mut data_paths, paths);
    }
    if let Some(paths) = cfg.get("data-local") {
        add_paths(&mut data_paths, paths);
    }
    data_paths.push(
        ini_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("Data Files"),
    );

    for i in 0.. {
        let key = format!("{base}{i}");
        let Some(values) = ini.get(&key) else {
            break;
        };

        for entry in values {
            let ext = entry
                .get(entry.len().saturating_sub(3)..)
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext != "esm" && ext != "esp" {
                continue;
            }

            let mut found = false;
            for data_path in &data_paths {
                let path = data_path.join(entry);
                if let Some(time) = last_write_time(&path) {
                    content_files.push((time, path));
                    found = true;
                    break;
                }
            }
            let _ = found;
        }
    }

    content_files.sort_by_key(|(time, _)| *time);

    let mut unsorted: Vec<(String, Vec<String>)> = Vec::new();
    for (_, path) in &content_files {
        let masters = esm_header::read_master_dependencies(path).map_err(|e| e.to_string())?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        unsorted.push((filename, masters));
    }

    let mut sorted = dependency_sort(unsorted);
    fix_tribunal_bloodmoon_order(&mut sorted);

    cfg.remove("content");
    cfg.insert("content".to_string(), sorted);
    let _ = encoding;
    Ok(())
}

fn fix_tribunal_bloodmoon_order(sorted: &mut Vec<String>) {
    let Some(mw_idx) = find_string_ci(sorted, "Morrowind.esm") else {
        return;
    };
    let _ = mw_idx;

    let Some(tribunal_idx) = find_string_ci(sorted, "Tribunal.esm") else {
        return;
    };
    let Some(bloodmoon_idx) = find_string_ci(sorted, "Bloodmoon.esm") else {
        return;
    };

    if bloodmoon_idx >= tribunal_idx {
        return;
    }

    let tribunal = sorted[tribunal_idx].clone();
    sorted.remove(tribunal_idx);
    let bloodmoon_idx = find_string_ci(sorted, "Bloodmoon.esm").unwrap_or(bloodmoon_idx);
    sorted.insert(bloodmoon_idx, tribunal);
}

fn dependency_sort(mut source: Vec<(String, Vec<String>)>) -> Vec<String> {
    let mut result = Vec::new();
    while let Some((name, _)) = source.first().cloned() {
        dependency_sort_step(&name, &mut source, &mut result);
    }
    result
}

fn dependency_sort_step(
    element: &str,
    source: &mut Vec<(String, Vec<String>)>,
    result: &mut Vec<String>,
) {
    let Some(pos) = source.iter().position(|(name, _)| name == element) else {
        return;
    };
    let (_, deps) = source.remove(pos);
    for dep in deps {
        dependency_sort_step(&dep, source, result);
    }
    result.push(element.to_string());
}

fn find_string_ci(haystack: &[String], needle: &str) -> Option<usize> {
    haystack.iter().position(|s| s.eq_ignore_ascii_case(needle))
}

fn add_paths(output: &mut Vec<PathBuf>, input: &[String]) {
    for path in input {
        let trimmed = path.trim();
        let unquoted = trimmed
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(trimmed);
        output.push(PathBuf::from(unquoted));
    }
}

fn last_write_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn insert_multistrmap(cfg: &mut MultiStrMap, key: &str, value: &str) {
    cfg.entry(key.to_string())
        .or_default()
        .push(value.to_string());
}

pub fn write_to_file(writer: &mut impl Write, cfg: &MultiStrMap) -> std::io::Result<()> {
    for (key, values) in cfg {
        for value in values {
            writeln!(writer, "{key}={value}")?;
        }
    }
    Ok(())
}

pub fn quote_data_path(path: &Path) -> String {
    let native = path.to_string_lossy();
    if native.contains(' ') {
        format!("\"{native}\"")
    } else {
        native.into_owned()
    }
}

/// Find a plugin file under one or more `data=` paths (searches subdirectories).
pub fn find_plugin_in_data_paths(
    data_paths: &[PathBuf],
    plugin_name: &str,
) -> Option<PathBuf> {
    PluginIndex::build(data_paths)
        .find(plugin_name)
        .cloned()
}

/// Sort plugin filenames by file timestamp and master dependencies (OpenMW ini importer rules).
/// Extended OpenMW content (`.omwscripts`, `.omwaddon`, `.bsa`) keeps the load-order sequence
/// from `plugin_names` and is appended after dependency-sorted `.esm`/`.esp` entries.
pub fn sort_content_plugins(
    index: &PluginIndex,
    plugin_names: &[String],
) -> Result<Vec<String>, String> {
    let mut record_files: Vec<(SystemTime, PathBuf)> = Vec::new();
    for name in plugin_names {
        if !is_record_plugin(name) {
            continue;
        }

        if let Some(path) = index.find(name) {
            if let Some(time) = last_write_time(path) {
                record_files.push((time, path.clone()));
            }
        }
    }

    record_files.sort_by_key(|(time, _)| *time);

    let mut unsorted: Vec<(String, Vec<String>)> = Vec::new();
    for (_, path) in &record_files {
        let masters = esm_header::read_master_dependencies(path).map_err(|e| e.to_string())?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        unsorted.push((filename, masters));
    }

    let mut sorted = dependency_sort(unsorted);
    fix_tribunal_bloodmoon_order(&mut sorted);

    let mut seen: std::collections::HashSet<String> = sorted
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();

    for name in plugin_names {
        if is_record_plugin(name) || !content_files::is_openmw_content_file(name) {
            continue;
        }
        if index.find(name).is_none() {
            continue;
        }
        if seen.insert(name.to_ascii_lowercase()) {
            sorted.push(name.clone());
        }
    }

    Ok(sorted)
}

/// Resolve `Morrowind.ini` from a Data Files directory (onboarding stores Data Files path).
pub fn resolve_morrowind_ini(data_files_path: &Path) -> Option<PathBuf> {
    let in_data = data_files_path.join("Morrowind.ini");
    if in_data.exists() {
        return Some(in_data);
    }
    data_files_path
        .parent()
        .map(|p| p.join("Morrowind.ini"))
        .filter(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_ini_parses_sections() {
        let dir = std::env::temp_dir().join(format!("nerevar-ini-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let ini = dir.join("Morrowind.ini");
        let mut f = fs::File::create(&ini).unwrap();
        writeln!(f, "[General]").unwrap();
        writeln!(f, "sTest=1").unwrap();
        writeln!(f, "[Game Files]").unwrap();
        writeln!(f, "GameFile0=Morrowind.esm").unwrap();

        let map = load_ini_file(&ini, IniEncoding::Win1252).unwrap();
        assert_eq!(map.get("General:sTest").map(|v| v[0].as_str()), Some("1"));
        assert_eq!(
            map.get("Game Files:GameFile0").map(|v| v[0].as_str()),
            Some("Morrowind.esm")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sort_content_plugins_includes_extended_openmw_files() {
        let dir = std::env::temp_dir().join(format!("nerevar-sort-ext-{}", std::process::id()));
        let mod_dir = dir.join("Lua Pack");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(mod_dir.join("Helper.esp"), b"").unwrap();
        fs::write(mod_dir.join("pack.omwscripts"), b"GLOBAL: x.lua").unwrap();

        let index = PluginIndex::build(&[mod_dir.clone()]);
        let sorted = sort_content_plugins(
            &index,
            &[
                "pack.omwscripts".into(),
                "Helper.esp".into(),
            ],
        )
        .unwrap();

        assert!(
            sorted.iter().any(|name| name.eq_ignore_ascii_case("Helper.esp")),
            "expected Helper.esp, got {sorted:?}"
        );
        assert!(
            sorted
                .iter()
                .any(|name| name.eq_ignore_ascii_case("pack.omwscripts")),
            "expected pack.omwscripts, got {sorted:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_archives_adds_morrowind_bsa() {
        let mut ini = MultiStrMap::new();
        ini.insert(
            "Archives:Archive 0".to_string(),
            vec!["Tribunal.bsa".to_string()],
        );
        let mut cfg = MultiStrMap::new();
        import_archives(&mut cfg, &ini);
        let archives = cfg.get("fallback-archive").unwrap();
        assert_eq!(archives[0], "Morrowind.bsa");
        assert!(archives.contains(&"Tribunal.bsa".to_string()));
    }
}
