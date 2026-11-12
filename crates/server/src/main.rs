//! Runs the sync server. See `README.md` for the environment variables.

use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sabi_server=debug")))
        .init();

    if let Err(err) = run().await {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let config = sabi_server::config::Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .map_err(|e| format!("could not connect to the database: {e}"))?;
    sabi_server::migrate(&pool).await.map_err(|e| format!("migration failed: {e}"))?;

    let state = sabi_server::AppState::new(pool, &config.jwt_secret);
    let app = sabi_server::app(state, config.allowed_origins);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port))
        .await
        .map_err(|e| format!("could not listen on port {}: {e}", config.port))?;
    tracing::info!("listening on http://localhost:{}", config.port);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
        })
        .await
        .map_err(|e| e.to_string())
}
