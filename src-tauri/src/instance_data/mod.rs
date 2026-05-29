pub mod commands;

mod checksum;
mod load_order;
mod lookup;
mod manifest;
mod manifest_compare;
mod openmw_cfg;
mod paths;
mod resolver;
mod scan;
mod types;

pub use load_order::{load_load_order, save_load_order, scan_and_merge_load_order};
pub use lookup::find_instance_by_id;
pub use manifest::{
    build_manifest, load_manifest, validate_manifest_against_disk,
};
pub use manifest_compare::manifests_differ;
pub use openmw_cfg::{
    resolve_instance_openmw_config, write_ephemeral_openmw_cfg, write_tes3mp_launch_openmw_cfg,
};
pub use paths::{
    ensure_instance_data_layout, manifest_path, package_abs_path, resolve_package_data_dir,
};
pub use resolver::{resolve_load_order, resolve_synced_load_order};
pub use types::*;

#[cfg(test)]
mod bindings {
    use super::types::{LoadOrder, NerevarManifest, RequiredDataFileEntry, ScannedPackage};
    use crate::data::{InstanceConfig, NewConnectionConfig};
    use crate::process_manager::types::ProcessRole;
    use crate::sync_client::types::{
        ProcessOutputEvent, ProcessStatusEvent, ProcessStream, RemoteManifestSummary,
        SyncPhase, SyncProgressEvent,
    };
    use crate::sync_host::SyncHostStatus;
    use ts_rs::{Config, TS};

    /// Run with `cargo test export_bindings` to refresh `src/types/*.ts`.
    #[test]
    fn export_bindings() {
        let cfg = Config::default();
        NerevarManifest::export_all(&cfg).expect("export manifest graph");
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
    }
}
