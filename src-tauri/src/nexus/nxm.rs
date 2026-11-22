//! nxm:// links: the "Mod Manager Download" buttons of the Nexus Mods website.
//!
//! When players choose Nerevar for these links (Nerevar's settings), clicking one starts Nerevar, which passes the
//! link to the running Nerevar (single instance). The link carries a key that lets free accounts download that file.
//! A link is used by the modlist install waiting for that file, else it installs the mod into the instance whose
//! data manager is open.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::sync::oneshot;
use ts_rs::TS;

pub const SCHEME: &str = "nxm";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxmLink {
    pub game: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<String>,
}

/// nxm://morrowind/mods/55064/files/1000047019?key=XXXX&expires=XXXX&user_id=XXXX
pub fn parse(link: &str) -> Option<NxmLink> {
    let rest = link.trim().trim_matches('"').strip_prefix("nxm://")?;
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let [game, "mods", mod_id, "files", file_id] = parts.as_slice() else {
        return None;
    };
    let mut link = NxmLink {
        game: game.to_ascii_lowercase(),
        mod_id: mod_id.parse().ok()?,
        file_id: file_id.parse().ok()?,
        key: None,
        expires: None,
    };
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("key", value)) => link.key = Some(value.to_string()),
            Some(("expires", value)) => link.expires = Some(value.to_string()),
            _ => {}
        }
    }
    Some(link)
}

/// Where nxm links go.
#[derive(Default)]
struct Router {
    /// Modlist installs waiting for the player to click a file's download button: (mod, file) -> waiter
    waiting: HashMap<(u64, u64), oneshot::Sender<NxmLink>>,
    /// The instance whose data manager is open
    target_instance: Option<String>,
}

fn router() -> &'static Mutex<Router> {
    static ROUTER: OnceLock<Mutex<Router>> = OnceLock::new();
    ROUTER.get_or_init(|| Mutex::new(Router::default()))
}

/// Waits for the link of a file, for modlist installs with free accounts.
pub fn wait_for(mod_id: u64, file_id: u64) -> oneshot::Receiver<NxmLink> {
    let (sender, receiver) = oneshot::channel();
    if let Ok(mut router) = router().lock() {
        router.waiting.insert((mod_id, file_id), sender);
    }
    receiver
}

pub fn stop_waiting(mod_id: u64, file_id: u64) {
    if let Ok(mut router) = router().lock() {
        router.waiting.remove(&(mod_id, file_id));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NexusDownloadEvent {
    pub instance_id: Option<String>,
    pub mod_id: u64,
    pub file_id: u64,
    /// "started", "installed", "failed" or "unhandled"
    pub state: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

fn emit(app: &AppHandle, event: NexusDownloadEvent) {
    let _ = app.emit("nexus-download", event);
}

/// Handles an nxm link from the command line of this or a second Nerevar.
pub fn handle(app: &AppHandle, raw: &str) {
    let Some(link) = parse(raw) else {
        return;
    };
    tauri_plugin_log::log::info!("Nexus download link for mod {} file {}", link.mod_id, link.file_id);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }

    let (waiter, target) = match router().lock() {
        Ok(mut router) => (router.waiting.remove(&(link.mod_id, link.file_id)), router.target_instance.clone()),
        Err(_) => return,
    };
    if let Some(waiter) = waiter {
        let _ = waiter.send(link);
        return;
    }
    let Some(instance_id) = target else {
        emit(app, NexusDownloadEvent {
            instance_id: None,
            mod_id: link.mod_id,
            file_id: link.file_id,
            state: "unhandled".into(),
            message: "Open an instance's data manager to install Nexus downloads into it".into(),
            package: None,
        });
        return;
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        emit(&app, NexusDownloadEvent {
            instance_id: Some(instance_id.clone()),
            mod_id: link.mod_id,
            file_id: link.file_id,
            state: "started".into(),
            message: "Downloading from Nexus Mods".into(),
            package: None,
        });
        let result = crate::packages::install_nexus_link(&app, &instance_id, &link).await;
        let (state, message, package) = match result {
            Ok(package) => ("installed", format!("Installed {package}"), Some(package)),
            Err(error) => ("failed", error, None),
        };
        emit(&app, NexusDownloadEvent {
            instance_id: Some(instance_id),
            mod_id: link.mod_id,
            file_id: link.file_id,
            state: state.into(),
            message,
            package,
        });
    });
}

/// Handles nxm links among a Nerevar command line.
pub fn handle_args(app: &AppHandle, args: impl IntoIterator<Item = String>) {
    for arg in args {
        if arg.trim_matches('"').starts_with("nxm://") {
            handle(app, &arg);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NexusLinkHandlerStatus {
    /// Whether nxm links open Nerevar
    pub registered: bool,
    pub supported: bool,
}

#[tauri::command]
pub fn nexus_link_handler_status(app: AppHandle) -> NexusLinkHandlerStatus {
    match app.deep_link().is_registered(SCHEME) {
        Ok(registered) => NexusLinkHandlerStatus { registered, supported: true },
        Err(_) => NexusLinkHandlerStatus { registered: false, supported: false },
    }
}

/// Makes nxm links open Nerevar, or stops it. Other mod managers (Mod Organizer 2, Vortex, umo) handle them again
/// when they register themselves.
#[tauri::command]
pub fn nexus_set_link_handler(app: AppHandle, enabled: bool) -> Result<NexusLinkHandlerStatus, String> {
    let result = if enabled { app.deep_link().register(SCHEME) } else { app.deep_link().unregister(SCHEME) };
    result.map_err(|e| format!("Couldn't change the nxm:// link handler: {e}"))?;
    Ok(nexus_link_handler_status(app))
}

/// The instance that receives Nexus downloads; the data manager sets it while it is open.
#[tauri::command]
pub fn nexus_set_download_target(instance_id: Option<String>) {
    if let Ok(mut router) = router().lock() {
        router.target_instance = instance_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_links() {
        let link = parse("nxm://Morrowind/mods/55064/files/1000047019?key=abc&expires=123&user_id=9").unwrap();
        assert_eq!(link.game, "morrowind");
        assert_eq!((link.mod_id, link.file_id), (55064, 1000047019));
        assert_eq!(link.key.as_deref(), Some("abc"));
        assert_eq!(link.expires.as_deref(), Some("123"));
        assert!(parse("nxm://morrowind/collections/abc").is_none());
        assert!(parse("https://example.com").is_none());
    }
}
