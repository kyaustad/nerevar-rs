use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::instance_settings::InstanceSettings;

/// 2: shader packages are marked with their `shader_kind`.
pub const LOAD_ORDER_VERSION: u32 = 2;
/// 2: TES3MP-NG. Adds the host's release, description, game address and client config files.
/// 3: shader packages are listed and synced separately, in `shaders`.
pub const MANIFEST_VERSION: u32 = 3;

/// Files of the host's TES3MP-NG `config/` folder that are seeded on synced clients, such as the
/// post-processing shader settings. Only plain names from this list are accepted.
pub const SYNCED_CLIENT_FILES: &[&str] = &["shaders.yaml"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClientConfigFile {
    /// File name in the TES3MP-NG `config/` folder, from `SYNCED_CLIENT_FILES`.
    pub name: String,
    pub contents: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PackageKind {
    Mod,
    Replacer,
}

/// Shader packages are kept apart from the data packages: they are listed and synced separately, and are
/// installed by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShaderPackageKind {
    /// Post-processing shaders (`shaders/*.omwfx` and their textures and scripts), which OpenMW loads from
    /// its data folders. They are added after the data packages.
    PostProcessing,
    /// Replacements for OpenMW's own shaders (`resources/shaders`, such as PBR shaders). OpenMW only reads
    /// those from its install, so they are copied over the install's shaders instead of being a data folder.
    CoreShaders,
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
    /// Set for shader packages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader_kind: Option<ShaderPackageKind>,
}

impl LoadOrderEntry {
    pub fn is_core_shaders(&self) -> bool {
        self.shader_kind == Some(ShaderPackageKind::CoreShaders)
    }
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
    /// Set for the packages in `NerevarManifest::shaders`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader_kind: Option<ShaderPackageKind>,
}

/// The instance's shader packages, synced after the data packages.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShaderManifest {
    pub packages: Vec<ManifestPackage>,
    pub total_download_bytes: u64,
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
    #[serde(default)]
    pub instance_description: String,
    /// The host's TES3MP-NG release tag (`v0.51.0`). Clients must run the same build to connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tes3mp_release_tag: Option<String>,
    pub generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_game_data: Option<String>,
    pub packages: Vec<ManifestPackage>,
    pub resolved: ResolvedOpenMwConfig,
    pub total_download_bytes: u64,
    /// TES3MP game server port from the host's `tes3mp-server-default.cfg` [General] section.
    #[serde(default = "default_tes3mp_server_port")]
    pub tes3mp_server_port: u16,
    /// Address of the TES3MP game server when it isn't the sync host's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tes3mp_server_address: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tes3mp_server_password: String,
    #[serde(default = "default_required_data_files")]
    pub required_data_files: Vec<RequiredDataFileEntry>,
    #[serde(default)]
    pub instance_settings: InstanceSettings,
    /// Seeded into clients' `config/` folder (see `SYNCED_CLIENT_FILES`).
    #[serde(default)]
    pub client_files: Vec<ClientConfigFile>,
    /// Shader packages, listed and synced apart from `packages`.
    #[serde(default)]
    pub shaders: ShaderManifest,
}

impl NerevarManifest {
    /// Data and shader packages.
    pub fn all_packages(&self) -> impl Iterator<Item = &ManifestPackage> {
        self.packages.iter().chain(self.shaders.packages.iter())
    }

    /// The manifest with only its data packages, or only its shader packages, for syncing each on its own.
    pub fn sync_section(&self, section: SyncSection) -> NerevarManifest {
        let mut view = self.clone();
        view.shaders = ShaderManifest::default();
        if section == SyncSection::Shaders {
            view.packages = self.shaders.packages.clone();
            view.total_download_bytes = self.shaders.total_download_bytes;
        }
        view
    }
}

/// The parts of an instance that are synced one after the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSection {
    Data,
    Shaders,
}

impl SyncSection {
    /// The host's route for the section's files.
    pub fn route(self) -> &'static str {
        match self {
            SyncSection::Data => "packages",
            SyncSection::Shaders => "shaders",
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader_kind: Option<ShaderPackageKind>,
}
