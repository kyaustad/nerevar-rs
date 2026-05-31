use std::path::{Path, PathBuf};

use super::types::InstanceSettings;

const CONFIG_LUA: &str = "config.lua";
const SCRIPTS_SUBDIR: &str = "server/scripts";

pub fn find_tes3mp_config_lua(tes3mp_dir: &Path) -> Result<PathBuf, String> {
    let direct = tes3mp_dir.join(SCRIPTS_SUBDIR).join(CONFIG_LUA);
    if direct.is_file() {
        return Ok(direct);
    }

    find_file_by_name(tes3mp_dir, CONFIG_LUA, 6).ok_or_else(|| {
        format!(
            "Could not find {CONFIG_LUA} under {}/{}",
            tes3mp_dir.display(),
            SCRIPTS_SUBDIR
        )
    })
}

fn find_file_by_name(dir: &Path, file_name: &str, max_depth: u32) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }

    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(file_name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file_by_name(&path, file_name, max_depth - 1) {
                return Some(found);
            }
        }
    }

    None
}

pub fn write_tes3mp_game_settings(
    tes3mp_dir: &Path,
    settings: &InstanceSettings,
) -> Result<PathBuf, String> {
    let path = find_tes3mp_config_lua(tes3mp_dir)?;
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let updated = patch_game_settings_block(&contents, settings);
    std::fs::write(&path, updated).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(path)
}

fn patch_game_settings_block(contents: &str, settings: &InstanceSettings) -> String {
    let marker = "config.gameSettings";
    let Some(start) = contents.find(marker) else {
        return append_game_settings_block(contents, settings);
    };

    let Some(open_brace) = contents[start..].find('{') else {
        return append_game_settings_block(contents, settings);
    };

    let block_start = start + open_brace;
    let Some(close_brace) = find_matching_brace(contents, block_start) else {
        return append_game_settings_block(contents, settings);
    };

    let mut out = String::new();
    out.push_str(&contents[..start]);
    out.push_str(&render_game_settings_block(settings));
    out.push_str(&contents[close_brace + 1..]);
    out
}

fn append_game_settings_block(contents: &str, settings: &InstanceSettings) -> String {
    let mut out = contents.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&render_game_settings_block(settings));
    out
}

fn render_game_settings_block(settings: &InstanceSettings) -> String {
    let mut out = String::from("config.gameSettings = {\n");
    for entry in &settings.tes3mp_game_settings {
        out.push_str(&format!(
            "    {{ name = \"{}\", value = {} }},\n",
            escape_lua_string(&entry.name),
            entry.value.as_lua_literal()
        ));
    }
    out.push_str("}\n");
    out
}

fn find_matching_brace(contents: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in contents[open_index..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open_index + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn escape_lua_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_settings::defaults::default_instance_settings;

    #[test]
    fn replaces_existing_game_settings_block() {
        let input = r#"config = {}
config.gameSettings = {
    { name = "best attack", value = true },
}
config.other = 1
"#;
        let settings = default_instance_settings();
        let output = patch_game_settings_block(input, &settings);
        assert!(output.contains("config.gameSettings = {"));
        assert!(!output.contains("value = true"));
        assert!(output.contains("best attack"));
        assert!(output.contains("config.other = 1"));
    }
}
