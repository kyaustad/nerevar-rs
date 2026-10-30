use std::sync::Mutex;

use tauri::State;

use crate::instance_data::{find_instance_by_id, resolve_package_data_dir};
use crate::AppState;

use super::{instance_lua_mods, LuaModInfo};

/// The instance's Lua mods, with suggested modes for their global scripts.
#[tauri::command]
pub async fn scan_instance_lua_mods(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<Vec<LuaModInfo>, String> {
    let instance = {
        let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
        find_instance_by_id(&guard.nerevar_config, &instance_id)
            .ok_or_else(|| format!("Instance not found: {instance_id}"))?
            .clone()
    };
    let data_dir = resolve_package_data_dir(&instance);
    tauri::async_runtime::spawn_blocking(move || instance_lua_mods(&data_dir))
        .await
        .map_err(|error| format!("Lua mod scan failed: {error}"))?
}
