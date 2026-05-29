use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncHostStatus {
    pub sync_port: u32,
    pub server_online: bool,
    pub hosting_instance_id: Option<String>,
    pub hosting_instance_name: Option<String>,
    pub manifest_available: bool,
}
