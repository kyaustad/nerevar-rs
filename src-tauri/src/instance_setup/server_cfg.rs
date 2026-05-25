use std::path::{Path, PathBuf};

use crate::data::NewInstanceConfig;
use tauri_plugin_log::log::info;

const SERVER_DEFAULTS_CFG: &str = "tes3mp-server-default.cfg";
pub const INSTANCE_TES3MP_DIR: &str = "tes3mp";

pub fn instance_tes3mp_dir(instance_root: &Path) -> PathBuf {
    instance_root.join(INSTANCE_TES3MP_DIR)
}

pub fn create_instance_data_dir(instance_root: &Path) -> Result<(), String> {
    let data_dir = instance_root.join("data");
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory at {}: {e}", data_dir.display()))?;
    info!("Created instance data directory at {}", data_dir.display());
    Ok(())
}

pub fn apply_server_defaults(
    tes3mp_dir: &Path,
    settings: &NewInstanceConfig,
) -> Result<(), String> {
    let cfg_path = find_server_defaults_cfg(tes3mp_dir)?;
    info!(
        "Applying server defaults to {}",
        cfg_path.display()
    );

    let contents =
        std::fs::read_to_string(&cfg_path).map_err(|e| format!("Failed to read {}: {e}", cfg_path.display()))?;

    let updated = patch_server_defaults_cfg(&contents, settings);

    std::fs::write(&cfg_path, updated)
        .map_err(|e| format!("Failed to write {}: {e}", cfg_path.display()))?;

    Ok(())
}

fn find_server_defaults_cfg(tes3mp_dir: &Path) -> Result<PathBuf, String> {
    let direct = tes3mp_dir.join(SERVER_DEFAULTS_CFG);
    if direct.is_file() {
        return Ok(direct);
    }

    find_file_by_name(tes3mp_dir, SERVER_DEFAULTS_CFG, 4)
        .ok_or_else(|| {
            format!(
                "Could not find {SERVER_DEFAULTS_CFG} under {}",
                tes3mp_dir.display()
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CfgSection {
    None,
    General,
    Plugins,
    MasterServer,
}

fn parse_cfg_section(line: &str) -> Option<CfgSection> {
    match line.trim() {
        "[General]" => Some(CfgSection::General),
        "[Plugins]" => Some(CfgSection::Plugins),
        "[MasterServer]" => Some(CfgSection::MasterServer),
        _ => None,
    }
}

fn setting_key(line: &str) -> Option<&str> {
    let line = line.split('#').next()?.trim();
    if line.starts_with('[') {
        return None;
    }
    let (key, _) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

fn patch_server_defaults_cfg(contents: &str, settings: &NewInstanceConfig) -> String {
    let mut section = CfgSection::None;
    let mut lines: Vec<String> = Vec::new();

    for line in contents.lines() {
        if let Some(next) = parse_cfg_section(line) {
            section = next;
            lines.push(line.to_string());
            continue;
        }

        let Some(key) = setting_key(line) else {
            lines.push(line.to_string());
            continue;
        };

        let patched = match (section, key) {
            (CfgSection::General, "hostname") => {
                Some(format!("hostname = {}", settings.server_host_name))
            }
            (CfgSection::General, "maximumPlayers" | "players") => {
                Some(format!("maximumPlayers = {}", settings.max_players))
            }
            (CfgSection::General, "port") => {
                Some(format!("port = {}", settings.server_port))
            }
            (CfgSection::General, "password") => {
                Some(format!("password = {}", settings.password))
            }
            (CfgSection::MasterServer, "enabled") => Some(format!(
                "enabled = {}",
                if settings.master_server_enabled {
                    "true"
                } else {
                    "false"
                }
            )),
            _ => None,
        };

        lines.push(patched.unwrap_or_else(|| line.to_string()));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_general_and_master_server_sections() {
        let input = r#"[General]
hostname = Old Name
maximumPlayers = 10
port = 25565
password = old

[MasterServer]
enabled = false
"#;

        let settings = NewInstanceConfig {
            release_id: "1".to_string(),
            instance_name: "test".to_string(),
            instance_description: String::new(),
            instance_root_path: String::new(),
            server_host_name: "Nerevar Server".to_string(),
            max_players: 64,
            server_port: 25570,
            password: "secret".to_string(),
            master_server_enabled: true,
        };

        let output = patch_server_defaults_cfg(input, &settings);
        assert!(output.contains("hostname = Nerevar Server"));
        assert!(output.contains("maximumPlayers = 64"));
        assert!(output.contains("port = 25570"));
        assert!(output.contains("password = secret"));
        assert!(output.contains("enabled = true"));
    }

    #[test]
    fn patch_default_layout_keeps_master_server_port() {
        let input = r#"[General]
localAddress = 0.0.0.0
port = 25565
maximumPlayers = 64
hostname = TES3MP server
logLevel = 1
password =

[Plugins]
home = ./server
plugins = serverCore.lua

[MasterServer]
enabled = true
address = master.tes3mp.com
port = 25561
rate = 10000
"#;

        let settings = NewInstanceConfig {
            release_id: "1".to_string(),
            instance_name: "test".to_string(),
            instance_description: String::new(),
            instance_root_path: String::new(),
            server_host_name: "My Server".to_string(),
            max_players: 32,
            server_port: 25570,
            password: String::new(),
            master_server_enabled: false,
        };

        let output = patch_server_defaults_cfg(input, &settings);

        let general_port = output
            .lines()
            .skip_while(|l| l.trim() != "[General]")
            .skip(1)
            .take_while(|l| !l.trim().starts_with('['))
            .find(|l| setting_key(l) == Some("port"))
            .expect("general port");
        assert_eq!(general_port.trim(), "port = 25570");

        let master_port = output
            .lines()
            .skip_while(|l| l.trim() != "[MasterServer]")
            .skip(1)
            .find(|l| setting_key(l) == Some("port"))
            .expect("master port");
        assert_eq!(master_port.trim(), "port = 25561");
        assert!(output.contains("enabled = false"));
    }
}
