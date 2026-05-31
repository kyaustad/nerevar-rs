mod health;
mod sync;

use std::sync::Arc;

use axum::Router;

use crate::nerevar_server::state::ServerContext;

/// Application HTTP routes. Add new modules here and `.merge()` them in.
pub fn router(ctx: Arc<ServerContext>) -> Router {
    Router::new()
        .merge(health::router())
        .merge(sync::router().with_state(ctx))
}
