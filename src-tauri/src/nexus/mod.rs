//! Nexus Mods account and API (v1), for downloading the mods of Modding-OpenMW.com lists.
//!
//! Players link their account with their personal API key (https://next.nexusmods.com/settings/api-keys). The key
//! is kept in the operating system's credential store (Windows Credential Manager, macOS Keychain, Secret Service),
//! never in Nerevar's files. Premium accounts download directly; free accounts download through the "Mod Manager
//! Download" buttons of the Nexus website, whose nxm:// links Nerevar handles (see `nxm`).

pub mod nxm;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

const API_BASE: &str = "https://api.nexusmods.com/v1";
const KEYRING_SERVICE: &str = "Nerevar";
const KEYRING_USER: &str = "nexus-api-key";
pub const APP_NAME: &str = "Nerevar";

/// The account Nerevar is linked to.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NexusAccountStatus {
    pub linked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub premium: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NexusUser {
    pub name: String,
    #[serde(default)]
    pub is_premium: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NexusFile {
    pub file_id: u64,
    pub name: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub category_id: u64,
    #[serde(default)]
    pub category_name: Option<String>,
    #[serde(default)]
    pub size_in_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NexusMod {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NexusFileUpdate {
    pub old_file_id: u64,
    pub new_file_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NexusFileList {
    pub files: Vec<NexusFile>,
    #[serde(default)]
    pub file_updates: Vec<NexusFileUpdate>,
}

#[derive(Debug, Clone, Deserialize)]
struct NexusDownloadLink {
    #[serde(rename = "URI")]
    uri: String,
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| format!("Credential store unavailable: {e}"))
}

pub fn stored_api_key() -> Option<String> {
    keyring_entry().ok()?.get_password().ok().filter(|key| !key.trim().is_empty())
}

fn store_api_key(key: &str) -> Result<(), String> {
    keyring_entry()?
        .set_password(key)
        .map_err(|e| format!("Failed to save the Nexus API key: {e}"))
}

fn delete_api_key() -> Result<(), String> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to remove the Nexus API key: {e}")),
    }
}

pub fn user_agent() -> String {
    format!("{APP_NAME}/{}", env!("CARGO_PKG_VERSION"))
}

/// Nexus Mods API client with the player's key.
#[derive(Clone)]
pub struct NexusClient {
    http: reqwest::Client,
    key: String,
}

impl NexusClient {
    pub fn new(key: String) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .user_agent(user_agent())
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { http, key })
    }

    pub fn from_stored_key() -> Option<Self> {
        Self::new(stored_api_key()?).ok()
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, String> {
        let response = self
            .http
            .get(format!("{API_BASE}/{path}"))
            .header("apikey", &self.key)
            .header("Application-Name", APP_NAME)
            .header("Application-Version", env!("CARGO_PKG_VERSION"))
            .send()
            .await
            .map_err(|e| format!("Nexus Mods request failed: {e}"))?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err("Nexus Mods rejected the API key; link your account again".into());
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err("Nexus Mods rate limit reached; try again later".into());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Nexus Mods returned {status}: {}", body.chars().take(200).collect::<String>()));
        }
        response.json::<T>().await.map_err(|e| format!("Unexpected Nexus Mods response: {e}"))
    }

    pub async fn validate(&self) -> Result<NexusUser, String> {
        self.get("users/validate.json").await
    }

    pub async fn files(&self, game: &str, mod_id: u64) -> Result<NexusFileList, String> {
        self.get(&format!("games/{game}/mods/{mod_id}/files.json")).await
    }

    /// Download link of a file. Only Premium accounts get links without the key of an nxm:// link.
    pub async fn download_link(&self, game: &str, mod_id: u64, file_id: u64) -> Result<String, String> {
        self.download_link_with_key(game, mod_id, file_id, None, None).await
    }

    pub async fn download_link_with_key(
        &self,
        game: &str,
        mod_id: u64,
        file_id: u64,
        key: Option<&str>,
        expires: Option<&str>,
    ) -> Result<String, String> {
        let mut path = format!("games/{game}/mods/{mod_id}/files/{file_id}/download_link.json");
        if let (Some(key), Some(expires)) = (key, expires) {
            path.push_str(&format!("?key={}&expires={}", urlencoding::encode(key), urlencoding::encode(expires)));
        }
        let links: Vec<NexusDownloadLink> = self.get(&path).await?;
        links.into_iter().next().map(|link| link.uri).ok_or_else(|| "Nexus Mods returned no download link".into())
    }

    pub async fn mod_details(&self, game: &str, mod_id: u64) -> Result<NexusMod, String> {
        self.get(&format!("games/{game}/mods/{mod_id}.json")).await
    }
}

async fn account_status() -> NexusAccountStatus {
    let Some(client) = NexusClient::from_stored_key() else {
        return NexusAccountStatus::default();
    };
    match client.validate().await {
        Ok(user) => NexusAccountStatus { linked: true, name: Some(user.name), premium: user.is_premium, error: None },
        Err(error) => NexusAccountStatus { linked: true, error: Some(error), ..Default::default() },
    }
}

async fn link_key(key: String) -> Result<NexusAccountStatus, String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Paste your Nexus Mods API key".into());
    }
    NexusClient::new(key.clone())?.validate().await?;
    store_api_key(&key)?;
    Ok(account_status().await)
}

#[tauri::command]
pub async fn nexus_account_status() -> Result<NexusAccountStatus, String> {
    Ok(account_status().await)
}

/// Links the account with a personal API key from https://next.nexusmods.com/settings/api-keys.
#[tauri::command]
pub async fn nexus_link_api_key(api_key: String) -> Result<NexusAccountStatus, String> {
    link_key(api_key).await
}

#[tauri::command]
pub async fn nexus_unlink() -> Result<NexusAccountStatus, String> {
    delete_api_key()?;
    Ok(account_status().await)
}
