use std::path::Path;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::instance_data::{load_manifest, manifest_path};
use crate::instance_setup::{instance_tes3mp_dir, read_tes3mp_server_settings};
use crate::nerevar_server::state::ServerContext;
use crate::sync_auth::{sync_password_matches, SYNC_PASSWORD_HEADER};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestSummary {
    instance_id: String,
    instance_name: String,
    total_download_bytes: u64,
    package_count: u32,
    tes3mp_server_port: u16,
    password_required: bool,
    packages: Vec<ManifestPackageSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestPackageSummary {
    id: String,
    name: String,
    relative_dir: String,
    total_size_bytes: u64,
    file_count: u32,
}

pub fn router() -> Router<Arc<ServerContext>> {
    Router::new()
        .route("/", get(root_manifest_summary))
        .route("/manifest", get(get_full_manifest))
        .route("/download", post(acknowledge_download))
        .route(
            "/packages/{package_id}/files/{*file_path}",
            get(serve_package_file),
        )
}

async fn hosting_data_dir(
    state: &ServerContext,
) -> Result<std::path::PathBuf, (StatusCode, String)> {
    let host = state
        .sync_host
        .lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Lock poisoned".to_string()))?;

    let data_dir = host
        .hosting_data_dir
        .clone()
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "No instance is hosting sync".to_string(),
        ))?;

    Ok(data_dir)
}

fn verify_sync_password(
    state: &ServerContext,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, String)> {
    let instance_root = {
        let host = state
            .sync_host
            .lock()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Lock poisoned".to_string()))?;
        host.hosting_instance_root.clone().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "No instance is hosting sync".to_string(),
        ))?
    };

    let tes3mp_dir = instance_tes3mp_dir(&instance_root);
    let settings = read_tes3mp_server_settings(&tes3mp_dir).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read TES3MP server password: {error}"),
        )
    })?;

    let provided = headers
        .get(SYNC_PASSWORD_HEADER)
        .and_then(|value| value.to_str().ok());

    if sync_password_matches(&settings.password, provided) {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            "Sync password required or incorrect".to_string(),
        ))
    }
}

fn password_required(state: &ServerContext) -> Result<bool, (StatusCode, String)> {
    let instance_root = {
        let host = state
            .sync_host
            .lock()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Lock poisoned".to_string()))?;
        host.hosting_instance_root.clone().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "No instance is hosting sync".to_string(),
        ))?
    };

    let tes3mp_dir = instance_tes3mp_dir(&instance_root);
    let settings = read_tes3mp_server_settings(&tes3mp_dir).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read TES3MP server password: {error}"),
        )
    })?;

    Ok(crate::sync_auth::sync_password_required(&settings.password))
}

async fn root_manifest_summary(
    State(state): State<Arc<ServerContext>>,
    headers: HeaderMap,
) -> Result<Json<ManifestSummary>, (StatusCode, String)> {
    verify_sync_password(&state, &headers)?;
    let data_dir = hosting_data_dir(&state).await?;
    let manifest = load_manifest(&data_dir).map_err(|e| (StatusCode::NOT_FOUND, e))?;
    let password_required = password_required(&state)?;

    let packages: Vec<ManifestPackageSummary> = manifest
        .packages
        .iter()
        .map(|p| ManifestPackageSummary {
            id: p.id.clone(),
            name: p.name.clone(),
            relative_dir: p.relative_dir.clone(),
            total_size_bytes: p.total_size_bytes,
            file_count: p.file_count,
        })
        .collect();

    Ok(Json(ManifestSummary {
        instance_id: manifest.instance_id,
        instance_name: manifest.instance_name,
        total_download_bytes: manifest.total_download_bytes,
        package_count: packages.len() as u32,
        tes3mp_server_port: manifest.tes3mp_server_port,
        password_required,
        packages,
    }))
}

async fn get_full_manifest(
    State(state): State<Arc<ServerContext>>,
    headers: HeaderMap,
) -> Result<Json<crate::instance_data::NerevarManifest>, (StatusCode, String)> {
    verify_sync_password(&state, &headers)?;
    let data_dir = hosting_data_dir(&state).await?;
    let manifest = load_manifest(&data_dir).map_err(|e| (StatusCode::NOT_FOUND, e))?;
    Ok(Json(manifest))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadAck {
    message: String,
    manifest_path: String,
}

async fn acknowledge_download(
    State(state): State<Arc<ServerContext>>,
    headers: HeaderMap,
) -> Result<Json<DownloadAck>, (StatusCode, String)> {
    verify_sync_password(&state, &headers)?;
    let data_dir = hosting_data_dir(&state).await?;
    let path = manifest_path(&data_dir);
    Ok(Json(DownloadAck {
        message: "Download acknowledged. Fetch files per package from /packages/{id}/files/...".to_string(),
        manifest_path: path.to_string_lossy().into_owned(),
    }))
}

async fn serve_package_file(
    State(state): State<Arc<ServerContext>>,
    headers: HeaderMap,
    AxumPath((package_id, file_path)): AxumPath<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    verify_sync_password(&state, &headers)?;
    let data_dir = hosting_data_dir(&state).await?;
    let manifest = load_manifest(&data_dir).map_err(|e| (StatusCode::NOT_FOUND, e))?;

    let package = manifest
        .packages
        .iter()
        .find(|p| p.id == package_id)
        .ok_or((StatusCode::NOT_FOUND, "Package not found".to_string()))?;

    let relative = file_path.replace('\\', "/");
    if relative.contains("..") {
        return Err((StatusCode::BAD_REQUEST, "Invalid file path".to_string()));
    }

    let allowed = package.files.iter().any(|f| f.path == relative);
    if !allowed {
        return Err((StatusCode::NOT_FOUND, "File not in manifest".to_string()));
    }

    let full_path = Path::new(&data_dir).join(&package.relative_dir).join(&relative);
    let bytes = std::fs::read(&full_path)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Failed to read file: {e}")))?;

    Ok(bytes)
}
