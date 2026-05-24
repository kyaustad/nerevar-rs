use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/ping", get(ping))
}

async fn root() -> &'static str {
    "Hello from your mums house"
}

async fn health() -> &'static str {
    "ok"
}

async fn ping() -> &'static str {
    "pong"
}
