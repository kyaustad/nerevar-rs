use std::sync::Arc;

use crate::sync_host::{SharedHostingManifestCache, SharedSyncHost};

#[derive(Clone)]
pub struct ServerContext {
    pub sync_host: SharedSyncHost,
    pub manifest_cache: SharedHostingManifestCache,
}

impl ServerContext {
    pub fn new(sync_host: SharedSyncHost, manifest_cache: SharedHostingManifestCache) -> Arc<Self> {
        Arc::new(Self {
            sync_host,
            manifest_cache,
        })
    }
}
