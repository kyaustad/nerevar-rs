use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(TS, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub data_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_sync_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tes3mp_server_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_password: Option<String>,
}

#[derive(TS, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceEditPayload {
    pub id: String,
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: u16,
    pub password: String,
}

#[derive(TS, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceConnectionSettings {
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: u16,
    pub password: String,
    pub is_synced: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_port: Option<u16>,
}

#[derive(TS, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewConnectionConfig {
    pub release_id: String,
    pub connection_name: String,
    pub connection_description: String,
    pub instance_root_path: String,
    pub instance_data_dir: String,
    pub remote_host: String,
    pub remote_sync_port: u16,
    pub sync_password: String,
}

#[derive(Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewInstanceConfig {
    pub release_id: String,
    pub instance_name: String,
    pub instance_description: String,
    pub instance_root_path: String,
    pub instance_data_dir: String,
    pub server_host_name: String,
    pub max_players: u32,
    pub server_port: u16,
    pub password: String,
    pub master_server_enabled: bool,
}

#[derive(TS, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NerevarConfig {
    pub onboarding_complete: bool,
    pub owned_instances: Option<Vec<InstanceConfig>>,
    pub synced_instances: Option<Vec<InstanceConfig>>,
    pub root_path: Option<String>,
    pub sync_port: i32,
}

// impl Default for NerevarConfig {
//     fn default() -> Self {
//         Self {
//             onboarding_complete: false,
//             instances: None,
//             root_instance_path: None,
//             sync_port: 25567,
//         }
//     }
// }

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct GithubAssetResponse {
    pub url: String,
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub label: Option<String>,
    pub content_type: String,
    pub state: String,
    pub size: u64,
    pub download_count: u64,
    pub created_at: String,
    pub updated_at: String,
    pub browser_download_url: String,
}

#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct GithubReleaseResponse {
    pub url: String,
    pub assets_url: String,
    pub upload_url: String,
    pub html_url: String,
    pub id: u64,
    pub node_id: String,
    pub tag_name: String,
    pub target_commitish: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: String,
    pub published_at: String,
    pub assets: Vec<GithubAssetResponse>,
    pub tarball_url: String,
    pub zipball_url: String,
    pub body: String,
}
