mod routes;
pub mod state;

use std::sync::Arc;

use tauri_plugin_log::log::info;

pub async fn try_bind(port: i32) -> Result<tokio::net::TcpListener, String> {
    let addr = format!("0.0.0.0:{port}");
    tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|error| format!("Failed to bind to {addr}: {error}"))
}

pub async fn serve(
    listener: tokio::net::TcpListener,
    ctx: Arc<state::ServerContext>,
) -> Result<(), String> {
    let app = routes::router(ctx);
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    info!("NEREVAR SERVER: listening on {addr}");

    axum::serve(listener, app)
        .await
        .map_err(|error| error.to_string())
}

pub async fn start_web_server_on_port(
    port: i32,
    ctx: Arc<state::ServerContext>,
) -> Result<(), String> {
    let listener = try_bind(port).await?;
    serve(listener, ctx).await
}
