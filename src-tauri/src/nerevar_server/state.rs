use std::sync::Arc;

use crate::sync_host::SharedSyncHost;

#[derive(Clone)]
pub struct ServerContext {
    pub sync_host: SharedSyncHost,
}

impl ServerContext {
    pub fn new(sync_host: SharedSyncHost) -> Arc<Self> {
        Arc::new(Self { sync_host })
    }
}
