use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, State};

use super::install::{install_modlist, MomwInstallReport};
use super::{fetch_modlists, MomwModlistSummary};
use crate::instance_data::{find_instance_by_id, resolve_package_data_dir, ProgressEmitter};
use crate::AppState;

fn cancel_flag() -> &'static Mutex<Option<Arc<AtomicBool>>> {
    static FLAG: OnceLock<Mutex<Option<Arc<AtomicBool>>>> = OnceLock::new();
    FLAG.get_or_init(|| Mutex::new(None))
}

/// The lists on Modding-OpenMW.com (or its beta site).
#[tauri::command]
pub async fn momw_list_modlists(beta: Option<bool>) -> Result<Vec<MomwModlistSummary>, String> {
    let beta = beta.unwrap_or(false);
    Ok(fetch_modlists(beta).await?.into_iter().map(|slug| MomwModlistSummary { slug, beta }).collect())
}

/// Installs a Modding-OpenMW.com list into an instance owned by this player. Running it again resumes it and
/// updates mods whose files changed.
#[tauri::command]
pub async fn momw_install_modlist(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    slug: String,
    beta: Option<bool>,
    operation_id: Option<String>,
) -> Result<MomwInstallReport, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };
    if instance.remote_host.is_some() {
        return Err("Synced instances get their mods from their host".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut current = cancel_flag().lock().map_err(|_| "Lock poisoned".to_string())?;
        if current.is_some() {
            return Err("A modlist is being installed already".into());
        }
        *current = Some(cancel.clone());
    }
    let link_handler = crate::nexus::nxm::nexus_link_handler_status(app.clone()).registered;
    let progress = operation_id.map(|id| ProgressEmitter::new(app, instance_id.clone(), id));
    let result = install_modlist(
        resolve_package_data_dir(&instance),
        slug,
        beta.unwrap_or(false),
        link_handler,
        progress,
        cancel,
    )
    .await;
    if let Ok(mut current) = cancel_flag().lock() {
        *current = None;
    }
    result
}

#[tauri::command]
pub fn momw_cancel_install() {
    if let Ok(current) = cancel_flag().lock() {
        if let Some(flag) = current.as_ref() {
            flag.store(true, Ordering::Relaxed);
        }
    }
}
