use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ping", get(ping))
}

async fn health() -> &'static str {
    "ok"
}

async fn ping() -> &'static str {
    "pong"
}
