
use axum::{routing::get, Router};
use crate::config::nerevar_config::{nerevar_config_file_path, load_or_create_nerevar_config_at};
pub async fn start_web_server() -> Result<(), String> {
    let config_path = nerevar_config_file_path()?;
    let config = load_or_create_nerevar_config_at(&config_path)?;
    let port = config.sync_port;

    let web_server = Router::new().route("/", get(|| async { "Hello from Nerevar!" }));
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {addr}: {e}"))?;

    axum::serve(listener, web_server)
        .await
        .map_err(|e| e.to_string())
        
}