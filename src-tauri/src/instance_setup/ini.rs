//! Editing of TES3MP's INI-style config files (`[Section]` headers and `key = value` lines).
//!
//! Like OpenMW's settings parser, only whole lines starting with `#` are comments: a `#` after a
//! value is part of it (passwords can contain one).

/// Value of `key` in `[section]`, or None when the file doesn't set it.
pub fn get_value(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut current = String::new();
    for line in contents.lines() {
        if let Some(name) = section_name(line) {
            current = name.to_string();
            continue;
        }
        if current != section {
            continue;
        }
        if let Some((line_key, value)) = key_value(line) {
            if line_key == key {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Set `(section, key, value)` entries, keeping every other line and comment. Keys the file
/// doesn't have yet are added at the end of their section, and missing sections at the end.
pub fn set_values(contents: &str, updates: &[(&str, &str, String)]) -> String {
    let mut written = vec![false; updates.len()];
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();

    for line in contents.lines() {
        if let Some(name) = section_name(line) {
            insert_pending(&mut out, &current, updates, &mut written);
            current = name.to_string();
            out.push(line.to_string());
            continue;
        }

        if let Some((key, _)) = key_value(line) {
            if let Some(index) = updates
                .iter()
                .position(|(section, update_key, _)| *section == current && *update_key == key)
            {
                if !written[index] {
                    written[index] = true;
                    out.push(format!("{key} = {}", updates[index].2));
                }
                continue;
            }
        }
        out.push(line.to_string());
    }
    insert_pending(&mut out, &current, updates, &mut written);

    // Sections the file doesn't have yet
    let mut sections: Vec<&str> = Vec::new();
    for (index, (section, _, _)) in updates.iter().enumerate() {
        if !written[index] && !sections.contains(section) {
            sections.push(section);
        }
    }
    for section in sections {
        if out.last().is_some_and(|line| !line.trim().is_empty()) {
            out.push(String::new());
        }
        out.push(format!("[{section}]"));
        for (index, (update_section, key, value)) in updates.iter().enumerate() {
            if *update_section == section && !written[index] {
                written[index] = true;
                out.push(format!("{key} = {value}"));
            }
        }
    }

    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// Add the section's keys the file didn't have, after its last non-blank line.
fn insert_pending(
    out: &mut Vec<String>,
    section: &str,
    updates: &[(&str, &str, String)],
    written: &mut [bool],
) {
    if section.is_empty() {
        return;
    }
    let mut at = out.len();
    while at > 0 && out[at - 1].trim().is_empty() {
        at -= 1;
    }
    for (index, (update_section, key, value)) in updates.iter().enumerate() {
        if *update_section == section && !written[index] {
            written[index] = true;
            out.insert(at, format!("{key} = {value}"));
            at += 1;
        }
    }
}

fn section_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .map(str::trim)
}

fn key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
        return None;
    }
    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim();
    (!key.is_empty()).then_some((key, value.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_by_section() {
        let cfg = "[General]\nport = 25565\npassword = a#b\n\n[MasterServer]\nport = 25561\n";
        assert_eq!(get_value(cfg, "General", "port").as_deref(), Some("25565"));
        assert_eq!(get_value(cfg, "MasterServer", "port").as_deref(), Some("25561"));
        assert_eq!(get_value(cfg, "General", "password").as_deref(), Some("a#b"));
        assert_eq!(get_value(cfg, "General", "missing"), None);
    }

    #[test]
    fn updates_existing_keys_and_keeps_the_rest() {
        let cfg = "# comment\n[General]\nport = 25565\nlogLevel = 1\n\n[Chat]\nkeySay = Y\n";
        let out = set_values(cfg, &[("General", "port", "25570".into())]);
        assert_eq!(out, "# comment\n[General]\nport = 25570\nlogLevel = 1\n\n[Chat]\nkeySay = Y\n");
    }

    #[test]
    fn adds_missing_keys_and_sections() {
        let cfg = "[General]\nport = 25565\n\n[Chat]\nkeySay = Y\n";
        let out = set_values(
            cfg,
            &[
                ("General", "password", "secret".into()),
                ("MasterServer", "enabled", "false".into()),
            ],
        );
        assert_eq!(
            out,
            "[General]\nport = 25565\npassword = secret\n\n[Chat]\nkeySay = Y\n\n[MasterServer]\nenabled = false\n"
        );
        assert_eq!(set_values("", &[("General", "port", "1".into())]), "[General]\nport = 1\n");
    }
}
