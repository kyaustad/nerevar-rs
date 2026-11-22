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
    let downloads_dir = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        crate::config::nerevar_config::downloads_dir(&guard.nerevar_config)
    };
    let link_handler = crate::nexus::nxm::nexus_link_handler_status(app.clone()).registered;
    let progress = operation_id.map(|id| ProgressEmitter::new(app, instance_id.clone(), id));
    let result = install_modlist(
        resolve_package_data_dir(&instance),
        downloads_dir.clone(),
        slug.clone(),
        beta.unwrap_or(false),
        link_handler,
        progress,
        cancel,
    )
    .await;
    if let Ok(mut current) = cancel_flag().lock() {
        *current = None;
    }
    log_report(&instance.name, &slug, &downloads_dir, &result);
    result
}

/// Writes what an install did to Nerevar's log, for players reporting a broken list.
fn log_report(instance: &str, slug: &str, downloads: &std::path::Path, result: &Result<MomwInstallReport, String>) {
    use tauri_plugin_log::log::{info, warn};
    let report = match result {
        Ok(report) => report,
        Err(error) => {
            warn!("Modlist {slug} into {instance} failed: {error}");
            return;
        }
    };
    info!(
        "Modlist {slug} into {instance} (downloads in {}): {} installed, {} up to date, {} repaired, {} manual, {} failed",
        downloads.display(),
        report.installed,
        report.up_to_date,
        report.repaired.len(),
        report.manual.len(),
        report.failed.len()
    );
    for name in &report.repaired {
        info!("  repaired: {name}");
    }
    for item in &report.manual {
        warn!("  manual: {item:?}");
    }
    for item in &report.left_out {
        warn!("  left out: {} (needs {})", item.plugin, item.master);
    }
    for item in &report.failed {
        warn!("  failed: {item:?}");
    }
    for note in &report.notes {
        warn!("  note: {note}");
    }
}

/// The report of the instance's last modlist install, if there was one.
#[tauri::command]
pub fn momw_last_report(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<Option<MomwInstallReport>, String> {
    let data_dir = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        let instance = find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?;
        resolve_package_data_dir(instance)
    };
    Ok(super::install::last_report(&data_dir))
}

/// Adds an archive the player downloaded for a mod of a list (`MomwManualMod::key`) to the downloads folder; the
/// next install of the list installs it.
#[tauri::command]
pub async fn momw_add_manual_archive(
    state: State<'_, Mutex<AppState>>,
    key: String,
    archive: String,
) -> Result<(), String> {
    let downloads_dir = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        crate::config::nerevar_config::downloads_dir(&guard.nerevar_config)
    };
    tauri::async_runtime::spawn_blocking(move || {
        super::install::add_manual_archive(downloads_dir, &key, std::path::Path::new(&archive))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The downloads folder of modlist installs.
#[tauri::command]
pub fn momw_get_downloads_dir(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
    Ok(crate::config::nerevar_config::downloads_dir(&guard.nerevar_config).to_string_lossy().into_owned())
}

/// Chooses the downloads folder of modlist installs (none: Nerevar's own). Returns the folder now used.
#[tauri::command]
pub fn momw_set_downloads_dir(state: State<'_, Mutex<AppState>>, path: Option<String>) -> Result<String, String> {
    crate::config::nerevar_config::set_downloads_dir(state, path)
}

#[tauri::command]
pub fn momw_cancel_install() {
    if let Ok(current) = cancel_flag().lock() {
        if let Some(flag) = current.as_ref() {
            flag.store(true, Ordering::Relaxed);
        }
    }
}
