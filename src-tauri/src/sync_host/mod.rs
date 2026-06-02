pub mod commands;
mod manifest_cache;
mod status;

pub use commands::{activate_hosting_instance, clear_hosting_instance, get_sync_host_status};
pub use manifest_cache::{
    get_package_file_path, new_shared_hosting_manifest_cache, SharedHostingManifestCache,
};
pub use status::SyncHostStatus;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
pub struct SyncHostState {
    pub hosting_instance_id: Option<String>,
    pub hosting_data_dir: Option<PathBuf>,
    pub hosting_instance_root: Option<PathBuf>,
    /// Cached TES3MP sync password so file requests do not re-read server cfg every time.
    pub hosting_sync_password: Option<String>,
}

pub type SharedSyncHost = Arc<Mutex<SyncHostState>>;

pub fn new_shared_sync_host() -> SharedSyncHost {
    Arc::new(Mutex::new(SyncHostState::default()))
}
