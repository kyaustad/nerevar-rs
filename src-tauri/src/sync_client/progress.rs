use tauri::{AppHandle, Emitter};

use super::types::{SyncPhase, SyncProgressEvent};

/// Maps sync phase + byte progress to a single 0–100 value for the overall progress bar.
pub fn overall_sync_percent(phase: SyncPhase, bytes_done: u64, bytes_total: u64) -> u8 {
    match phase {
        SyncPhase::CheckingUpdates => 1,
        SyncPhase::VerifyingExisting => {
            if bytes_total == 0 {
                3
            } else {
                3 + percent_of_range(bytes_done, bytes_total, 7)
            }
        }
        SyncPhase::FetchingManifest => 10,
        SyncPhase::Downloading => {
            if bytes_total == 0 {
                12
            } else {
                12 + percent_of_range(bytes_done, bytes_total, 73)
            }
        }
        SyncPhase::ApplyingLoadOrder => 88,
        SyncPhase::Validating => {
            if bytes_total == 0 {
                92
            } else {
                92 + percent_of_range(bytes_done, bytes_total, 5)
            }
        }
        SyncPhase::WritingLaunchCfg => 98,
        SyncPhase::Complete => 100,
        SyncPhase::Cancelled | SyncPhase::Failed => {
            if bytes_total > 0 && bytes_done > 0 {
                percent_of_range(bytes_done, bytes_total, 85).max(1)
            } else {
                0
            }
        }
    }
}

fn percent_of_range(done: u64, total: u64, range: u8) -> u8 {
    if total == 0 {
        return 0;
    }
    let fraction = (done.min(total) as f64) / (total as f64);
    (fraction * f64::from(range)).round() as u8
}

#[allow(clippy::too_many_arguments)]
pub fn emit_sync_progress(
    app: &AppHandle,
    instance_id: &str,
    phase: SyncPhase,
    message: impl Into<String>,
    bytes_done: u64,
    bytes_total: u64,
    files_done: u64,
    files_total: u64,
    current_file: Option<String>,
) {
    let overall_percent = overall_sync_percent(phase.clone(), bytes_done, bytes_total);
    let _ = app.emit(
        "sync-progress",
        SyncProgressEvent {
            instance_id: instance_id.to_string(),
            phase,
            message: message.into(),
            bytes_done,
            bytes_total,
            files_done,
            files_total,
            overall_percent,
            current_file,
        },
    );
}
