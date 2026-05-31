use reqwest::Client;

use crate::instance_data::NerevarManifest;
use crate::sync_auth::SYNC_PASSWORD_HEADER;

use super::types::RemoteManifestSummary;

fn base_url(host: &str, port: u16) -> String {
    let host = host.trim().trim_end_matches('/');
    format!("http://{host}:{port}")
}

fn connection_error(context: &str, host: &str, port: u16, err: reqwest::Error) -> String {
    let url = format!("http://{host}:{port}");
    if err.is_connect() {
        format!(
            "{context} at {url}: could not connect. Ensure Nerevar is running and the sync server is listening on port {port}."
        )
    } else if err.is_timeout() {
        format!("{context} at {url}: request timed out.")
    } else {
        format!("{context} at {url}: {err}")
    }
}

fn apply_sync_password(
    builder: reqwest::RequestBuilder,
    sync_password: Option<&str>,
) -> reqwest::RequestBuilder {
    match sync_password.filter(|password| !password.is_empty()) {
        Some(password) => builder.header(SYNC_PASSWORD_HEADER, password),
        None => builder,
    }
}

pub async fn ping_nerevar_server(host: &str, port: u16) -> Result<(), String> {
    let client = Client::new();
    let url = format!("{}/ping", base_url(host, port));
    let response = client
        .get(&url)
        .header("User-Agent", "Nerevar-0.1.0")
        .send()
        .await
        .map_err(|e| connection_error("Failed to reach Nerevar server", host, port, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Nerevar server ping failed (HTTP {})",
            response.status()
        ));
    }
    Ok(())
}

pub async fn fetch_manifest_summary(
    host: &str,
    port: u16,
    sync_password: Option<&str>,
) -> Result<RemoteManifestSummary, String> {
    let client = Client::new();
    let url = format!("{}/", base_url(host, port));
    let request = apply_sync_password(
        client
            .get(&url)
            .header("User-Agent", "Nerevar-0.1.0"),
        sync_password,
    );
    let response = request
        .send()
        .await
        .map_err(|e| connection_error("Failed to fetch manifest summary", host, port, e))?;

    if response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE {
        return Err(
            "No instance is hosting sync yet. On the host instance, use \"Activate Nerevar syncing\" or save & host from the data manager.".to_string(),
        );
    }

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Sync password required or incorrect".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Manifest summary request failed (HTTP {status}): {body}"));
    }

    response
        .json::<RemoteManifestSummary>()
        .await
        .map_err(|e| format!("Invalid manifest summary response: {e}"))
}

pub async fn fetch_full_manifest(
    host: &str,
    port: u16,
    sync_password: Option<&str>,
) -> Result<NerevarManifest, String> {
    let client = Client::new();
    let url = format!("{}/manifest", base_url(host, port));
    let request = apply_sync_password(
        client
            .get(&url)
            .header("User-Agent", "Nerevar-0.1.0"),
        sync_password,
    );
    let response = request
        .send()
        .await
        .map_err(|e| connection_error("Failed to fetch manifest", host, port, e))?;

    if response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE {
        return Err(
            "No instance is hosting sync yet. On the host instance, use \"Activate Nerevar syncing\" or save & host from the data manager.".to_string(),
        );
    }

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Sync password required or incorrect".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Manifest request failed (HTTP {status}): {body}"));
    }

    response
        .json::<NerevarManifest>()
        .await
        .map_err(|e| format!("Invalid manifest response: {e}"))
}
