mod routes;

use crate::config::nerevar_config::{load_or_create_nerevar_config_at, nerevar_config_file_path};
use tauri_plugin_log::log::info;

pub async fn start_web_server() -> Result<(), String> {
    let config_path = nerevar_config_file_path()?;
    let config = load_or_create_nerevar_config_at(&config_path)?;
    let port = config.sync_port;

    let app = routes::router();
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {addr}: {e}"))?;

    info!("NEREVAR SERVER: listening on {addr}");

    axum::serve(listener, app)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
