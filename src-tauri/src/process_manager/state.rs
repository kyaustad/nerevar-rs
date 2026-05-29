use std::collections::HashMap;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::openmw_ini_importer::GlobalOpenMwLaunchPatch;
use crate::sync_client::types::ProcessStatusEvent;

use super::types::ProcessRole;

pub struct ManagedProcess {
    pub child: Arc<Mutex<Option<Child>>>,
}

pub struct ProcessManager {
    processes: Mutex<HashMap<String, ManagedProcess>>,
    global_openmw_patch: Mutex<Option<GlobalOpenMwLaunchPatch>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
            global_openmw_patch: Mutex::new(None),
        }
    }

    pub fn store_global_openmw_patch(
        &self,
        patch: GlobalOpenMwLaunchPatch,
    ) -> Result<(), String> {
        let mut guard = self
            .global_openmw_patch
            .lock()
            .map_err(|_| "Process manager lock poisoned".to_string())?;
        if guard.is_some() {
            return Err("Global OpenMW config is already patched for launch".to_string());
        }
        *guard = Some(patch);
        Ok(())
    }

    pub fn restore_global_openmw_patch_if_any(&self) {
        let patch = self
            .global_openmw_patch
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        if let Some(patch) = patch {
            if let Err(err) = crate::openmw_ini_importer::restore_global_openmw_launch_patch(patch)
            {
                tauri_plugin_log::log::error!(
                    "Failed to restore global OpenMW config after TES3MP launch: {err}"
                );
            }
        }
    }

    pub fn key(instance_id: &str, role: ProcessRole) -> String {
        format!("{}:{}", instance_id, role.as_str())
    }

    pub fn insert(
        &self,
        instance_id: &str,
        role: ProcessRole,
        child: Child,
    ) -> Result<Arc<Mutex<Option<Child>>>, String> {
        let key = Self::key(instance_id, role);
        let wrapped = Arc::new(Mutex::new(Some(child)));
        let mut guard = self
            .processes
            .lock()
            .map_err(|_| "Process manager lock poisoned".to_string())?;

        if let Some(existing) = guard.remove(&key) {
            stop_child_in_background(existing.child, None);
        }

        guard.insert(
            key,
            ManagedProcess {
                child: wrapped.clone(),
            },
        );
        Ok(wrapped)
    }

    /// Signal the process to exit without blocking the caller.
    pub fn stop(
        &self,
        app: Option<AppHandle>,
        instance_id: &str,
        role: ProcessRole,
    ) -> Result<bool, String> {
        let key = Self::key(instance_id, role);
        let child_arc = {
            let mut guard = self
                .processes
                .lock()
                .map_err(|_| "Process manager lock poisoned".to_string())?;
            guard.remove(&key).map(|managed| managed.child)
        };

        let Some(child_arc) = child_arc else {
            return Ok(false);
        };

        let instance_id = instance_id.to_string();
        let role_str = role.as_str().to_string();
        if role == ProcessRole::Client {
            self.restore_global_openmw_patch_if_any();
        }
        stop_child_in_background(
            child_arc,
            app.map(|handle| (handle, instance_id, role_str)),
        );
        Ok(true)
    }

    pub fn remove(&self, instance_id: &str, role: ProcessRole) {
        if let Ok(mut guard) = self.processes.lock() {
            guard.remove(&Self::key(instance_id, role));
        }
    }

    pub fn is_running(&self, instance_id: &str, role: ProcessRole) -> Result<bool, String> {
        let key = Self::key(instance_id, role);
        let guard = self
            .processes
            .lock()
            .map_err(|_| "Process manager lock poisoned".to_string())?;

        let Some(managed) = guard.get(&key) else {
            return Ok(false);
        };

        let child_arc = managed.child.clone();
        drop(guard);

        let mut slot = child_arc
            .lock()
            .map_err(|_| "Process child lock poisoned".to_string())?;

        let Some(ref mut child) = *slot else {
            return Ok(false);
        };

        match child.try_wait() {
            Ok(Some(_)) => {
                *slot = None;
                self.remove(instance_id, role);
                Ok(false)
            }
            Ok(None) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Kill and reap a child on a background thread so Tauri commands never block on `wait()`.
/// Only this path (or the watch thread after natural exit) may call `wait()`.
fn stop_child_in_background(
    child_arc: Arc<Mutex<Option<Child>>>,
    status_emit: Option<(AppHandle, String, String)>,
) {
    thread::spawn(move || {
        let exit_code = {
            let mut slot = match child_arc.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let Some(mut child) = slot.take() else {
                return;
            };
            let _ = child.kill();
            child.wait().ok().and_then(|s| s.code())
        };

        if let Some((app, instance_id, role)) = status_emit {
            let _ = app.emit(
                "process-status",
                ProcessStatusEvent {
                    instance_id,
                    role,
                    running: false,
                    exit_code,
                },
            );
        }
    });
}

/// Poll for process exit without holding the child lock across `wait()`.
pub fn spawn_exit_watcher(
    app: AppHandle,
    manager: Arc<ProcessManager>,
    instance_id: String,
    role: ProcessRole,
    child_arc: Arc<Mutex<Option<Child>>>,
) {
    thread::spawn(move || {
        let exit_code = loop {
            thread::sleep(Duration::from_millis(250));

            let status = {
                let mut slot = match child_arc.lock() {
                    Ok(g) => g,
                    Err(_) => break None,
                };
                let Some(ref mut child) = *slot else {
                    // Stopped via stop(); that path emits process-status.
                    return;
                };
                match child.try_wait() {
                    Ok(Some(status)) => {
                        *slot = None;
                        Some(status.code())
                    }
                    Ok(None) => continue,
                    Err(_) => {
                        *slot = None;
                        None
                    }
                }
            };

            if let Some(code) = status {
                break code;
            }
        };

        manager.remove(&instance_id, role);
        if role == ProcessRole::Client {
            manager.restore_global_openmw_patch_if_any();
        }
        let _ = app.emit(
            "process-status",
            ProcessStatusEvent {
                instance_id,
                role: role.as_str().to_string(),
                running: false,
                exit_code,
            },
        );
    });
}
