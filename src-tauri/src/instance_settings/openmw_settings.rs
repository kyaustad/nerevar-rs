use std::collections::BTreeMap;

use super::types::{InstanceSettings, SettingValue};

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
    out
}

pub fn merge_settings_overlay(base: &str, overlay: &str) -> String {
    let mut base_sections = parse_settings_cfg(base);
    let overlay_sections = parse_settings_cfg(overlay);

    for (section, entries) in overlay_sections {
        let target = base_sections.entry(section).or_default();
        for (key, value) in entries {
            target.insert(key, value);
        }
    }

    render_settings_cfg(&base_sections)
}

fn parse_settings_cfg(contents: &str) -> BTreeMap<String, BTreeMap<String, String>> {
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

fn render_settings_cfg(sections: &BTreeMap<String, BTreeMap<String, String>>) -> String {
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
    fn overlay_replaces_section_keys() {
        let base = "[Shaders]\nauto use object normal maps = false\nforce shaders = false\n";
        let overlay = "[Shaders]\nauto use object normal maps = true\n";
        let merged = merge_settings_overlay(base, overlay);
        assert!(merged.contains("auto use object normal maps = true"));
        assert!(merged.contains("force shaders = false"));
    }
}
