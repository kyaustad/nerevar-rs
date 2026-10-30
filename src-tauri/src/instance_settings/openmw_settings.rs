use std::collections::BTreeMap;

use super::types::InstanceSettings;

pub fn format_settings_overlay(settings: &InstanceSettings) -> String {
    let mut out = String::from("# Nerevar instance launch settings overlay\n");
    for (section, entries) in &settings.openmw_settings {
        if entries.is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(&format!("[{section}]\n"));
        for (key, value) in entries {
            out.push_str(&format!("{} = {}\n", key, value.as_settings_cfg_value()));
        }
    }

    let manual = settings.openmw_settings_cfg_overrides.trim();
    if !manual.is_empty() {
        out.push('\n');
        out.push_str("# Manual settings.cfg overrides\n");
        out.push_str(manual);
        if !manual.ends_with('\n') {
            out.push('\n');
        }
    }

    out
}

/// Sections of a `settings.cfg`, with their `key = value` settings.
pub fn parse_settings_cfg(contents: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut current = String::from("default");

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current = trimmed[1..trimmed.len() - 1].trim().to_string();
            sections.entry(current.clone()).or_default();
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        sections
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }

    sections
}

pub fn render_settings_cfg(sections: &BTreeMap<String, BTreeMap<String, String>>) -> String {
    let mut out = String::from("# Nerevar active OpenMW settings (managed during TES3MP launch)\n");
    for (section, entries) in sections {
        if entries.is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(&format!("[{section}]\n"));
        for (key, value) in entries {
            out.push_str(&format!("{key} = {value}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_settings_overlay_appends_manual_block() {
        use crate::instance_settings::InstanceSettings;

        let settings = InstanceSettings {
            version: 1,
            tes3mp_game_settings: vec![],
            openmw_settings: Default::default(),
            openmw_cfg_overrides: vec![],
            openmw_settings_cfg_overrides: "[Cells]\nviewing distance = 7168".into(),
        };
        let overlay = format_settings_overlay(&settings);
        assert!(overlay.contains("[Cells]"));
        assert!(overlay.contains("viewing distance = 7168"));
    }
}
