mod routes;
pub mod state;

use std::sync::Arc;

use tauri_plugin_log::log::info;

pub async fn start_web_server_on_port(
    port: i32,
    ctx: Arc<state::ServerContext>,
) -> Result<(), String> {
    let app = routes::router(ctx);
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
