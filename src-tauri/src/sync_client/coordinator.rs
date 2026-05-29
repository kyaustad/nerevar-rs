use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct SyncCoordinator {
    active: Mutex<Option<ActiveSync>>,
}

struct ActiveSync {
    pub instance_id: String,
    pub cancel: Arc<AtomicBool>,
}

impl SyncCoordinator {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
        }
    }

    pub fn begin(&self, instance_id: &str) -> Result<Arc<AtomicBool>, String> {
        let mut guard = self
            .active
            .lock()
            .map_err(|_| "Sync coordinator lock poisoned".to_string())?;

        if let Some(active) = guard.as_ref() {
            if active.instance_id != instance_id {
                return Err(format!(
                    "Another instance is already syncing ({})",
                    active.instance_id
                ));
            }
            active.cancel.store(true, Ordering::Relaxed);
        }

        let cancel = Arc::new(AtomicBool::new(false));
        *guard = Some(ActiveSync {
            instance_id: instance_id.to_string(),
            cancel: cancel.clone(),
        });
        Ok(cancel)
    }

    pub fn finish(&self, instance_id: &str) {
        if let Ok(mut guard) = self.active.lock() {
            if guard
                .as_ref()
                .is_some_and(|active| active.instance_id == instance_id)
            {
                *guard = None;
            }
        }
    }

    pub fn cancel(&self, instance_id: &str) -> bool {
        let guard = match self.active.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        if let Some(active) = guard.as_ref() {
            if active.instance_id == instance_id {
                active.cancel.store(true, Ordering::Relaxed);
                return true;
            }
        }
        false
    }
}
