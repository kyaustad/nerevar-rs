use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SyncPhase {
    CheckingUpdates,
    VerifyingExisting,
    FetchingManifest,
    Downloading,
    ApplyingLoadOrder,
    Validating,
    WritingLaunchCfg,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncProgressEvent {
    pub instance_id: String,
    pub phase: SyncPhase,
    pub message: String,
    pub bytes_done: u64,
    pub bytes_total: u64,
    #[serde(default)]
    pub files_done: u64,
    #[serde(default)]
    pub files_total: u64,
    #[serde(default)]
    pub overall_percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceSyncStatus {
    pub has_manifest: bool,
    pub can_resume: bool,
    pub is_complete: bool,
    pub bytes_verified: u64,
    pub bytes_total: u64,
    pub files_verified: u32,
    pub files_total: u32,
    #[serde(default)]
    pub percent_complete: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RemoteManifestSummary {
    pub instance_id: String,
    pub instance_name: String,
    pub total_download_bytes: u64,
    pub package_count: u32,
    pub tes3mp_server_port: u16,
    pub password_required: bool,
    pub packages: Vec<RemotePackageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RemotePackageSummary {
    pub id: String,
    pub name: String,
    pub relative_dir: String,
    pub total_size_bytes: u64,
    pub file_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ProcessStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProcessOutputEvent {
    pub instance_id: String,
    pub role: String,
    pub stream: ProcessStream,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProcessStatusEvent {
    pub instance_id: String,
    pub role: String,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}
