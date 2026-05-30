use std::path::Path;
use std::sync::Mutex;

use tauri::{Emitter, State};

use crate::instance_data::{
    build_manifest, delete_package, find_instance_by_id, import_mo2_modlist_from_csv,
    load_load_order, load_manifest, resolve_load_order, resolve_package_data_dir,
    save_load_order, scan_and_merge_load_order, validate_manifest_against_disk,
    write_ephemeral_openmw_cfg, LoadOrder, ManifestValidationResult, Mo2ModlistImportResult,
    NerevarManifest, ResolvedOpenMwConfig,
};
use crate::sync_host::SharedSyncHost;
use crate::AppState;

fn resolve_instance(
    state: &State<'_, Mutex<AppState>>,
    instance_id: &str,
) -> Result<crate::data::InstanceConfig, String> {
    let guard = state.lock().map_err(|_| "App state lock poisoned".to_string())?;
    find_instance_by_id(&guard.nerevar_config, instance_id)
        .ok_or_else(|| format!("Instance not found: {instance_id}"))
        .cloned()
}

/// Returns `(instance_id, name, instance_root, package_data_dir)`.
fn resolve_instance_data_dir(
    state: &State<'_, Mutex<AppState>>,
    instance_id: &str,
) -> Result<(String, String, std::path::PathBuf, std::path::PathBuf), String> {
    let instance = resolve_instance(state, instance_id)?;
    Ok((
        instance.id.clone(),
        instance.name.clone(),
        Path::new(&instance.path).to_path_buf(),
        resolve_package_data_dir(&instance),
    ))
}

#[tauri::command]
pub async fn scan_instance_data(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<LoadOrder, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    tauri::async_runtime::spawn_blocking(move || scan_and_merge_load_order(&data_dir))
        .await
        .map_err(|error| format!("Scan task failed: {error}"))?
}

#[tauri::command]
pub async fn import_mo2_modlist_csv(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    csv_path: String,
) -> Result<Mo2ModlistImportResult, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let load_order = scan_and_merge_load_order(&data_dir)?;
        let contents = std::fs::read_to_string(&csv_path)
            .map_err(|error| format!("Failed to read {}: {error}", csv_path))?;
        let import = import_mo2_modlist_from_csv(&load_order, &contents)?;
        Ok(Mo2ModlistImportResult {
            load_order: import.0,
            report: import.1,
        })
    })
    .await
    .map_err(|error| format!("Import task failed: {error}"))?
}

#[tauri::command]
pub fn get_instance_load_order(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<LoadOrder, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    load_load_order(&data_dir)
}

#[tauri::command]
pub fn save_instance_load_order(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    load_order: LoadOrder,
) -> Result<(), String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    save_load_order(&data_dir, &load_order)
}

#[tauri::command]
pub fn delete_instance_package(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    entry_id: String,
) -> Result<LoadOrder, String> {
    let (id, name, instance_root, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    delete_package(&id, &name, &instance_root, &data_dir, &entry_id)
}

#[tauri::command]
pub fn resolve_instance_openmw(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<ResolvedOpenMwConfig, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let load_order = load_load_order(&data_dir)?;
    resolve_load_order(&data_dir, &load_order)
}

#[tauri::command]
pub fn write_instance_launch_cfg(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<String, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let load_order = load_load_order(&data_dir)?;
    let resolved = resolve_load_order(&data_dir, &load_order)?;
    write_ephemeral_openmw_cfg(&data_dir, &resolved)
}

#[tauri::command]
pub fn build_instance_manifest(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<NerevarManifest, String> {
    let (id, name, instance_root, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let load_order = load_load_order(&data_dir)?;
    build_manifest(&id, &name, &instance_root, &data_dir, &load_order)
}

#[tauri::command]
pub fn validate_instance_manifest(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<ManifestValidationResult, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let manifest = load_manifest(&data_dir)?;
    Ok(validate_manifest_against_disk(&data_dir, &manifest))
}

#[tauri::command]
pub fn set_hosting_instance(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
    sync_host: State<'_, SharedSyncHost>,
    instance_id: String,
) -> Result<NerevarManifest, String> {
    let (id, name, instance_root, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let load_order = load_load_order(&data_dir)?;
    let manifest = build_manifest(&id, &name, &instance_root, &data_dir, &load_order)?;

    let mut host = sync_host
        .lock()
        .map_err(|_| "Sync host lock poisoned".to_string())?;
    host.hosting_instance_id = Some(instance_id);
    host.hosting_data_dir = Some(data_dir);

    let _ = app.emit("hosting-changed", ());
    Ok(manifest)
}
