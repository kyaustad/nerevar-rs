use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use crate::instance_data::{
    build_manifest, delete_package, find_instance_by_id, import_mo2_modlist_from_csv,
    load_load_order, load_manifest, resolve_load_order, resolve_package_data_dir,
    save_load_order, scan_and_merge_load_order, validate_manifest_against_disk,
    write_ephemeral_openmw_cfg, BackgroundOperationPhase, LoadOrder,
    ManifestValidationResult, Mo2ModlistImportResult, NerevarManifest, ProgressEmitter,
    ResolvedOpenMwConfig,
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

fn progress_emitter(
    app: AppHandle,
    instance_id: &str,
    operation_id: Option<String>,
) -> Option<ProgressEmitter> {
    operation_id.map(|id| ProgressEmitter::new(app, instance_id.to_string(), id))
}

#[tauri::command]
pub async fn scan_instance_data(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    operation_id: Option<String>,
) -> Result<LoadOrder, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let progress_instance_id = instance_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut progress = progress_emitter(app, &progress_instance_id, operation_id);
        scan_and_merge_load_order(&data_dir, &mut progress)
    })
    .await
    .map_err(|error| format!("Scan task failed: {error}"))?
}

#[tauri::command]
pub async fn import_mo2_modlist_csv(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    csv_path: String,
    operation_id: Option<String>,
) -> Result<Mo2ModlistImportResult, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let progress_instance_id = instance_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut progress = progress_emitter(app, &progress_instance_id, operation_id);
        let load_order = scan_and_merge_load_order(&data_dir, &mut progress)?;
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::ParsingCsv,
                "Parsing MO2 modlist CSV",
                0,
                1,
                None,
                true,
            );
        }
        let contents = std::fs::read_to_string(&csv_path)
            .map_err(|error| format!("Failed to read {}: {error}", csv_path))?;
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::ApplyingLoadOrder,
                "Applying MO2 priorities and plugin order",
                0,
                1,
                None,
                true,
            );
        }
        let import = import_mo2_modlist_from_csv(&load_order, &contents)?;
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::SavingLoadOrder,
                "Saving load-order.json",
                0,
                1,
                None,
                true,
            );
        }
        save_load_order(&data_dir, &import.0)?;
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
pub async fn save_instance_load_order(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    load_order: LoadOrder,
    operation_id: Option<String>,
) -> Result<(), String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let progress_instance_id = instance_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut progress = progress_emitter(app, &progress_instance_id, operation_id);
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::SavingLoadOrder,
                "Saving load-order.json",
                0,
                1,
                None,
                true,
            );
        }
        save_load_order(&data_dir, &load_order)
    })
    .await
    .map_err(|error| format!("Save task failed: {error}"))?
}

#[tauri::command]
pub async fn delete_instance_package(
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
pub async fn write_instance_launch_cfg(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
    operation_id: Option<String>,
) -> Result<String, String> {
    let (_, _, _, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let progress_instance_id = instance_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut progress = progress_emitter(app, &progress_instance_id, operation_id);
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::LoadingLoadOrder,
                "Loading load-order.json",
                0,
                1,
                None,
                true,
            );
        }
        let load_order = load_load_order(&data_dir)?;
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::ResolvingLoadOrder,
                "Resolving load order and plugin paths",
                0,
                1,
                None,
                true,
            );
        }
        let resolved = resolve_load_order(&data_dir, &load_order)?;
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::WritingLaunchCfg,
                "Writing nerevar-launch.cfg",
                0,
                1,
                None,
                true,
            );
        }
        write_ephemeral_openmw_cfg(&data_dir, &resolved)
    })
    .await
    .map_err(|error| format!("Launch cfg task failed: {error}"))?
}

#[tauri::command]
pub fn build_instance_manifest(
    state: State<'_, Mutex<AppState>>,
    instance_id: String,
) -> Result<NerevarManifest, String> {
    let (id, name, instance_root, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let load_order = load_load_order(&data_dir)?;
    let mut no_progress = None;
    build_manifest(
        &id,
        &name,
        &instance_root,
        &data_dir,
        &load_order,
        &mut no_progress,
    )
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
pub async fn save_and_host_instance(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    sync_host: State<'_, SharedSyncHost>,
    instance_id: String,
    load_order: LoadOrder,
    operation_id: Option<String>,
) -> Result<NerevarManifest, String> {
    let (id, name, instance_root, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let data_dir_for_host = data_dir.clone();
    let instance_root_for_host = instance_root.clone();
    let progress_instance_id = instance_id.clone();
    let app_for_host = app.clone();
    let manifest = tauri::async_runtime::spawn_blocking(move || {
        let mut progress = progress_emitter(app, &progress_instance_id, operation_id);
        if let Some(emitter) = progress.as_mut() {
            emitter.emit(
                BackgroundOperationPhase::SavingLoadOrder,
                "Saving load-order.json",
                0,
                1,
                None,
                true,
            );
        }
        save_load_order(&data_dir, &load_order)?;
        build_manifest(
            &id,
            &name,
            &instance_root,
            &data_dir,
            &load_order,
            &mut progress,
        )
    })
    .await
    .map_err(|error| format!("Host task failed: {error}"))??;

    let mut host = sync_host
        .lock()
        .map_err(|_| "Sync host lock poisoned".to_string())?;
    host.hosting_instance_id = Some(instance_id);
    host.hosting_data_dir = Some(data_dir_for_host);
    host.hosting_instance_root = Some(instance_root_for_host);

    let _ = app_for_host.emit("hosting-changed", ());
    Ok(manifest)
}

#[tauri::command]
pub async fn set_hosting_instance(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    sync_host: State<'_, SharedSyncHost>,
    instance_id: String,
    operation_id: Option<String>,
) -> Result<NerevarManifest, String> {
    let (id, name, instance_root, data_dir) = resolve_instance_data_dir(&state, &instance_id)?;
    let data_dir_for_host = data_dir.clone();
    let instance_root_for_host = instance_root.clone();
    let load_order = load_load_order(&data_dir)?;
    let progress_instance_id = instance_id.clone();
    let app_for_host = app.clone();
    let manifest = tauri::async_runtime::spawn_blocking(move || {
        let mut progress = progress_emitter(app, &progress_instance_id, operation_id);
        build_manifest(
            &id,
            &name,
            &instance_root,
            &data_dir,
            &load_order,
            &mut progress,
        )
    })
    .await
    .map_err(|error| format!("Host task failed: {error}"))??;

    let mut host = sync_host
        .lock()
        .map_err(|_| "Sync host lock poisoned".to_string())?;
    host.hosting_instance_id = Some(instance_id);
    host.hosting_data_dir = Some(data_dir_for_host);
    host.hosting_instance_root = Some(instance_root_for_host);

    let _ = app_for_host.emit("hosting-changed", ());
    Ok(manifest)
}
