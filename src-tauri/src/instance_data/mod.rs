pub mod commands;

mod checksum;
mod load_order;
mod lookup;
mod manifest;
mod manifest_compare;
mod mo2_modlist;
mod openmw_cfg;
mod paths;
mod progress;
mod prune;
mod remove;
mod resolver;
mod scan;
mod types;

pub use load_order::{load_load_order, save_load_order, scan_and_merge_load_order};
pub use lookup::find_instance_by_id;
pub use mo2_modlist::{import_mo2_modlist_from_csv, Mo2ModlistImportReport, Mo2ModlistImportResult};
pub use manifest::{
    build_manifest, load_manifest, validate_manifest_against_disk,
};
pub use manifest_compare::manifests_differ;
pub use openmw_cfg::{
    resolve_instance_openmw_config, write_ephemeral_openmw_cfg, write_instance_launch_cfg,
};
pub use progress::{
    BackgroundOperationPhase, BackgroundOperationProgressEvent, ProgressEmitter,
};
pub use paths::{
    ensure_instance_data_layout, launch_cfg_dir, launch_cfg_path, launch_settings_overlay_path,
    manifest_path, nerevar_dir, package_abs_path, resolve_package_data_dir,
};
pub use prune::prune_local_against_manifest;
pub use remove::delete_package;
pub use resolver::{resolve_load_order, resolve_synced_load_order};
pub use types::*;

#[cfg(test)]
mod bindings {
    use super::types::{LoadOrder, NerevarManifest, RequiredDataFileEntry, ScannedPackage};
    use crate::instance_data::{
        BackgroundOperationPhase, BackgroundOperationProgressEvent, Mo2ModlistImportReport,
        Mo2ModlistImportResult,
    };
    use crate::data::{InstanceConfig, NewConnectionConfig};
    use crate::process_manager::types::ProcessRole;
    use crate::sync_client::types::{
        ProcessOutputEvent, ProcessStatusEvent, ProcessStream, RemoteManifestSummary,
        SyncPhase, SyncProgressEvent,
    };
    use crate::sync_host::SyncHostStatus;
    use crate::instance_settings::{
        InstanceSettings, SettingCategory, SettingDefinition, SettingValue, SettingValueType,
        Tes3mpGameSettingEntry,
    };
    use ts_rs::{Config, TS};

    /// Run with `cargo test export_bindings` to refresh `src/types/*.ts`.
    #[test]
    fn export_bindings() {
        let cfg = Config::default();
        NerevarManifest::export_all(&cfg).expect("export manifest graph");
        Mo2ModlistImportReport::export_all(&cfg).expect("export Mo2ModlistImportReport");
        Mo2ModlistImportResult::export_all(&cfg).expect("export Mo2ModlistImportResult");
        LoadOrder::export_all(&cfg).expect("export load-order graph");
        ScannedPackage::export(&cfg).expect("export ScannedPackage");
        InstanceConfig::export_all(&cfg).expect("export InstanceConfig");
        NewConnectionConfig::export_all(&cfg).expect("export NewConnectionConfig");
        RemoteManifestSummary::export_all(&cfg).expect("export RemoteManifestSummary");
        SyncProgressEvent::export_all(&cfg).expect("export SyncProgressEvent");
        SyncPhase::export(&cfg).expect("export SyncPhase");
        ProcessOutputEvent::export_all(&cfg).expect("export ProcessOutputEvent");
        ProcessStatusEvent::export_all(&cfg).expect("export ProcessStatusEvent");
        ProcessStream::export(&cfg).expect("export ProcessStream");
        ProcessRole::export(&cfg).expect("export ProcessRole");
        RequiredDataFileEntry::export_all(&cfg).expect("export RequiredDataFileEntry");
        SyncHostStatus::export_all(&cfg).expect("export SyncHostStatus");
        BackgroundOperationProgressEvent::export_all(&cfg)
            .expect("export BackgroundOperationProgressEvent");
        BackgroundOperationPhase::export(&cfg).expect("export BackgroundOperationPhase");
        InstanceSettings::export_all(&cfg).expect("export InstanceSettings");
        SettingDefinition::export_all(&cfg).expect("export SettingDefinition");
        SettingValue::export_all(&cfg).expect("export SettingValue");
        SettingValueType::export(&cfg).expect("export SettingValueType");
        SettingCategory::export(&cfg).expect("export SettingCategory");
        Tes3mpGameSettingEntry::export_all(&cfg).expect("export Tes3mpGameSettingEntry");
    }
}
