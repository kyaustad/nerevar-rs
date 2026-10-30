//! TES3MP-NG's client and server settings.
//!
//! TES3MP-NG installs are portable: `tes3mp-*-default.cfg` next to the executables come with the
//! release, and the install's own `config/tes3mp-*.cfg` override them. Nerevar only writes the
//! values it manages into the `config/` files, so the defaults stay untouched by release updates and
//! players' other settings (chat, log level...) are kept.

use std::path::{Path, PathBuf};

use crate::data::NewInstanceConfig;
use tauri_plugin_log::log::info;

use super::ini::{get_value, set_values};

const SERVER_CFG: &str = "tes3mp-server";
const CLIENT_CFG: &str = "tes3mp-client";
pub const INSTANCE_TES3MP_DIR: &str = "tes3mp";
/// The portable install's config folder (`openmw.cfg`, `settings.cfg`, `tes3mp-*.cfg`, logs).
pub const TES3MP_CONFIG_DIR: &str = "config";

pub fn instance_tes3mp_dir(instance_root: &Path) -> PathBuf {
    instance_root.join(INSTANCE_TES3MP_DIR)
}

pub fn tes3mp_config_dir(tes3mp_dir: &Path) -> PathBuf {
    tes3mp_dir.join(TES3MP_CONFIG_DIR)
}

pub fn create_instance_data_dir(instance_data_dir: &Path) -> Result<(), String> {
    if instance_data_dir.exists() {
        info!("Instance data directory already exists, skipping creation!");
        return Ok(());
    }
    std::fs::create_dir_all(instance_data_dir).map_err(|e| {
        format!(
            "Failed to create data directory at {}: {e}",
            instance_data_dir.display()
        )
    })?;
    info!(
        "Created instance data directory at {}",
        instance_data_dir.display()
    );
    Ok(())
}

/// A TES3MP config: the release's `<name>-default.cfg` with the install's `config/<name>.cfg`.
struct Tes3mpCfg {
    default: PathBuf,
    user: PathBuf,
}

impl Tes3mpCfg {
    fn new(tes3mp_dir: &Path, name: &str) -> Self {
        Self {
            default: tes3mp_dir.join(format!("{name}-default.cfg")),
            user: tes3mp_config_dir(tes3mp_dir).join(format!("{name}.cfg")),
        }
    }

    fn read(path: &Path) -> Result<String, String> {
        if !path.is_file() {
            return Ok(String::new());
        }
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))
    }

    /// The effective value: the install's own setting, else the release default.
    fn get(&self, section: &str, key: &str) -> Result<Option<String>, String> {
        if !self.default.is_file() {
            return Err(format!(
                "{} not found: is TES3MP-NG installed?",
                self.default.display()
            ));
        }
        if let Some(value) = get_value(&Self::read(&self.user)?, section, key) {
            return Ok(Some(value));
        }
        Ok(get_value(&Self::read(&self.default)?, section, key))
    }

    fn set(&self, updates: &[(&str, &str, String)]) -> Result<PathBuf, String> {
        if let Some(parent) = self.user.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
        }
        let updated = set_values(&Self::read(&self.user)?, updates);
        std::fs::write(&self.user, updated)
            .map_err(|e| format!("Failed to write {}: {e}", self.user.display()))?;
        Ok(self.user.clone())
    }
}

pub fn apply_server_defaults(
    tes3mp_dir: &Path,
    settings: &NewInstanceConfig,
) -> Result<(), String> {
    let server = Tes3mpCfg::new(tes3mp_dir, SERVER_CFG);
    info!("Applying server settings to {}", server.user.display());
    server.set(&[
        ("General", "hostname", settings.server_host_name.clone()),
        ("General", "maximumPlayers", settings.max_players.to_string()),
        ("General", "port", settings.server_port.to_string()),
        ("General", "password", settings.password.clone()),
        (
            "MasterServer",
            "enabled",
            settings.master_server_enabled.to_string(),
        ),
    ])?;

    write_tes3mp_client_connection(
        tes3mp_dir,
        "127.0.0.1",
        settings.server_port,
        &settings.password,
    )?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tes3mpServerSettings {
    pub hostname: String,
    pub port: u16,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tes3mpClientSettings {
    pub destination_address: String,
    pub port: u16,
    pub password: String,
}

fn parse_port(value: Option<String>, what: &str) -> Result<u16, String> {
    let value = value.ok_or_else(|| format!("Could not find the {what} port"))?;
    value
        .parse()
        .map_err(|_| format!("Invalid {what} port: {value}"))
}

pub fn read_tes3mp_server_settings(tes3mp_dir: &Path) -> Result<Tes3mpServerSettings, String> {
    let server = Tes3mpCfg::new(tes3mp_dir, SERVER_CFG);
    Ok(Tes3mpServerSettings {
        hostname: server.get("General", "hostname")?.unwrap_or_default(),
        port: parse_port(server.get("General", "port")?, "TES3MP server")?,
        password: server.get("General", "password")?.unwrap_or_default(),
    })
}

pub fn read_tes3mp_client_settings(tes3mp_dir: &Path) -> Result<Tes3mpClientSettings, String> {
    let client = Tes3mpCfg::new(tes3mp_dir, CLIENT_CFG);
    Ok(Tes3mpClientSettings {
        destination_address: client
            .get("General", "destinationAddress")?
            .unwrap_or_default(),
        port: parse_port(client.get("General", "port")?, "TES3MP client")?,
        password: client.get("General", "password")?.unwrap_or_default(),
    })
}

pub fn update_server_connection_settings(
    tes3mp_dir: &Path,
    hostname: &str,
    port: u16,
    password: &str,
) -> Result<(), String> {
    Tes3mpCfg::new(tes3mp_dir, SERVER_CFG).set(&[
        ("General", "hostname", hostname.to_string()),
        ("General", "port", port.to_string()),
        ("General", "password", password.to_string()),
    ])?;
    Ok(())
}

/// Point the client at a game server, in the install's `config/tes3mp-client.cfg`.
pub fn write_tes3mp_client_connection(
    tes3mp_dir: &Path,
    destination_address: &str,
    port: u16,
    password: &str,
) -> Result<PathBuf, String> {
    Tes3mpCfg::new(tes3mp_dir, CLIENT_CFG).set(&[
        ("General", "destinationAddress", destination_address.to_string()),
        ("General", "port", port.to_string()),
        ("General", "password", password.to_string()),
    ])
}

/// Point this instance's client at its own local server.
pub fn write_owned_client_connection(tes3mp_dir: &Path) -> Result<(), String> {
    let server = read_tes3mp_server_settings(tes3mp_dir)?;
    write_tes3mp_client_connection(tes3mp_dir, "127.0.0.1", server.port, &server.password)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_install(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nerevar-cfg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tes3mp-server-default.cfg"),
            "[General]\nlocalAddress = 0.0.0.0\nport = 25565\nmaximumPlayers = 64\nhostname = TES3MP-NG server\npassword =\n\n[MasterServer]\nenabled = true\naddress = master.tes3mp.com\nport = 25561\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("tes3mp-client-default.cfg"),
            "[General]\ndestinationAddress = localhost\nport = 25565\npassword = SuperSecretPassword\n\n[Chat]\nkeySay = Y\nkeyChatMode = F8\n",
        )
        .unwrap();
        dir
    }

    fn new_instance(master_server_enabled: bool) -> NewInstanceConfig {
        NewInstanceConfig {
            release_id: "1".to_string(),
            instance_name: "test".to_string(),
            instance_description: String::new(),
            instance_root_path: String::new(),
            instance_data_dir: String::new(),
            server_host_name: "Nerevar Server".to_string(),
            max_players: 32,
            server_port: 25570,
            password: "secret".to_string(),
            master_server_enabled,
        }
    }

    #[test]
    fn server_settings_go_to_the_install_config_not_the_defaults() {
        let dir = temp_install("server");
        let default_before = std::fs::read_to_string(dir.join("tes3mp-server-default.cfg")).unwrap();

        apply_server_defaults(&dir, &new_instance(false)).unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.join("tes3mp-server-default.cfg")).unwrap(),
            default_before
        );
        let user = std::fs::read_to_string(dir.join("config/tes3mp-server.cfg")).unwrap();
        assert!(user.contains("hostname = Nerevar Server"));
        assert!(user.contains("maximumPlayers = 32"));
        assert!(user.contains("enabled = false"));
        assert!(!user.contains("address = master.tes3mp.com"));

        let settings = read_tes3mp_server_settings(&dir).unwrap();
        assert_eq!(settings.port, 25570);
        assert_eq!(settings.password, "secret");

        let client = read_tes3mp_client_settings(&dir).unwrap();
        assert_eq!(client.destination_address, "127.0.0.1");
        assert_eq!(client.port, 25570);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn client_connection_keeps_player_settings() {
        let dir = temp_install("client");
        std::fs::create_dir_all(dir.join("config")).unwrap();
        std::fs::write(dir.join("config/tes3mp-client.cfg"), "[Chat]\nkeyChatMode = F7\n").unwrap();

        write_tes3mp_client_connection(&dir, "example.com", 25570, "pw").unwrap();

        let user = std::fs::read_to_string(dir.join("config/tes3mp-client.cfg")).unwrap();
        assert!(user.contains("keyChatMode = F7"));
        assert!(user.contains("destinationAddress = example.com"));
        let client = read_tes3mp_client_settings(&dir).unwrap();
        assert_eq!((client.port, client.password.as_str()), (25570, "pw"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
