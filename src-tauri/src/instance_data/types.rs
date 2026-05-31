use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::instance_settings::InstanceSettings;

pub const LOAD_ORDER_VERSION: u32 = 1;
pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PackageKind {
    Mod,
    Replacer,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginEntry {
    pub file: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoadOrderEntry {
    pub id: String,
    pub name: String,
    pub kind: PackageKind,
    /// Path relative to instance data dir, e.g. `Better Bodies`.
    pub relative_dir: String,
    pub enabled: bool,
    pub priority: u32,
    #[serde(default)]
    pub plugins: Vec<PluginEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoadOrder {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_game_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_order: Option<Vec<String>>,
    pub entries: Vec<LoadOrderEntry>,
}

impl Default for LoadOrder {
    fn default() -> Self {
        Self {
            version: LOAD_ORDER_VERSION,
            base_game_data: None,
            content_order: None,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManifestFileEntry {
    pub path: String,
    pub size: u64,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManifestPackage {
    pub id: String,
    pub name: String,
    pub kind: PackageKind,
    pub relative_dir: String,
    pub priority: u32,
    pub tree_checksum: String,
    pub total_size_bytes: u64,
    pub file_count: u32,
    pub files: Vec<ManifestFileEntry>,
    #[serde(default)]
    pub plugins: Vec<PluginEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolvedOpenMwConfig {
    pub encoding: String,
    pub data_paths: Vec<String>,
    pub content: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NerevarManifest {
    pub version: u32,
    pub instance_id: String,
    pub instance_name: String,
    pub generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_game_data: Option<String>,
    pub packages: Vec<ManifestPackage>,
    pub resolved: ResolvedOpenMwConfig,
    pub total_download_bytes: u64,
    /// TES3MP game server port from the host's `tes3mp-server-default.cfg` [General] section.
    #[serde(default = "default_tes3mp_server_port")]
    pub tes3mp_server_port: u16,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tes3mp_server_password: String,
    #[serde(default = "default_required_data_files")]
    pub required_data_files: Vec<RequiredDataFileEntry>,
    #[serde(default)]
    pub instance_settings: InstanceSettings,
}

fn default_required_data_files() -> Vec<RequiredDataFileEntry> {
    Vec::new()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RequiredDataFileEntry {
    pub file: String,
    pub checksums: Vec<String>,
}

fn default_tes3mp_server_port() -> u16 {
    25565
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManifestValidationIssue {
    pub package_id: String,
    pub relative_dir: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManifestValidationResult {
    pub valid: bool,
    pub issues: Vec<ManifestValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScannedPackage {
    pub name: String,
    pub kind: PackageKind,
    pub relative_dir: String,
    pub plugins: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_checksum: Option<String>,
}
