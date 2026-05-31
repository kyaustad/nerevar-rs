use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BackgroundOperationPhase {
    ScanningPackages,
    MergingLoadOrder,
    SavingLoadOrder,
    ParsingCsv,
    ApplyingLoadOrder,
    LoadingLoadOrder,
    ResolvingLoadOrder,
    UpdatingServerMetadata,
    HashingPackage,
    HashingFiles,
    WritingManifest,
    WritingLaunchCfg,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackgroundOperationProgressEvent {
    pub operation_id: String,
    pub instance_id: String,
    pub phase: BackgroundOperationPhase,
    pub message: String,
    pub step: u64,
    pub total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_item: Option<String>,
}

pub struct ProgressEmitter {
    app: AppHandle,
    instance_id: String,
    operation_id: String,
    last_emit: Instant,
}

impl ProgressEmitter {
    pub fn new(app: AppHandle, instance_id: String, operation_id: String) -> Self {
        Self {
            app,
            instance_id,
            operation_id,
            last_emit: Instant::now() - Duration::from_secs(1),
        }
    }

    pub fn emit(
        &mut self,
        phase: BackgroundOperationPhase,
        message: impl Into<String>,
        step: u64,
        total: u64,
        current_item: Option<String>,
        force: bool,
    ) {
        if !force && self.should_throttle(phase) {
            return;
        }

        self.last_emit = Instant::now();
        let _ = self.app.emit(
            "background-operation-progress",
            BackgroundOperationProgressEvent {
                operation_id: self.operation_id.clone(),
                instance_id: self.instance_id.clone(),
                phase,
                message: message.into(),
                step,
                total,
                current_item,
            },
        );
    }

    fn should_throttle(&self, phase: BackgroundOperationPhase) -> bool {
        if !matches!(
            phase,
            BackgroundOperationPhase::HashingFiles | BackgroundOperationPhase::ScanningPackages
        ) {
            return false;
        }
        self.last_emit.elapsed() < Duration::from_millis(100)
    }
}
