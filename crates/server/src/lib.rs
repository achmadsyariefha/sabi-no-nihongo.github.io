//! Optional sync server for Sabi no Nihongo.
//!
//! The web app works fully offline; signing in to this server lets several
//! devices share one journal. Routes follow the old Express API under `/api`,
//! plus `POST /api/sync` for the browser's two-way sync.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod routes;

use std::sync::Arc;

use axum::http::{HeaderValue, Method, header};
use axum::Router;
use sqlx::PgPool;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub keys: Arc<auth::Keys>,
}

impl AppState {
    pub fn new(pool: PgPool, jwt_secret: &str) -> Self {
        Self { pool, keys: Arc::new(auth::Keys::new(jwt_secret)) }
    }
}

/// Applies the SQL migrations in `crates/server/migrations`.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// The full HTTP app. `origins` are the web pages allowed to call it.
pub fn app(state: AppState, origins: Vec<HeaderValue>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(3600));

    Router::new()
        .nest("/api", routes::router())
        .fallback(|| async { error::AppError::not_found("Not found") })
        .with_state(state)
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
