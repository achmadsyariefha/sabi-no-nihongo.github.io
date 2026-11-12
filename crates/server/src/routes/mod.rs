//! `/api` routes. Same paths as the Express backend, plus `/sync`.

mod account;
mod entries;
mod review;
mod srs;
mod stats;
mod sync;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::AppState;

/// Big enough for a first sync of a few thousand words.
const SYNC_BODY_LIMIT: usize = 10 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/auth/register", post(account::register))
        .route("/auth/login", post(account::login))
        .route("/auth/me", get(account::me).patch(account::update_me))
        .route("/entries", get(entries::list).post(entries::create))
        .route("/entries/{id}", get(entries::get_one).patch(entries::update).delete(entries::remove))
        .route("/stats", get(stats::get))
        .route("/review", get(review::get_session))
        .route("/review/reset", post(review::reset))
        .route("/review/mark", post(review::mark))
        .route("/srs/due", get(srs::due))
        .route("/srs/review", post(srs::review))
        .route("/sync", post(sync::sync).layer(DefaultBodyLimit::max(SYNC_BODY_LIMIT)))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
