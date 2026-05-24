mod health;

use axum::Router;

/// Application HTTP routes. Add new modules here and `.merge()` them in.
pub fn router() -> Router {
    Router::new().merge(health::router())
}
