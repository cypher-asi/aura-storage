mod handlers;
mod router;
mod state;

use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use aura_storage_auth::{InternalToken, TokenValidator};
use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("aura_storage=debug,tower_http=debug,info")),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let auth0_domain = std::env::var("AUTH0_DOMAIN").expect("AUTH0_DOMAIN must be set");
    let auth0_audience = std::env::var("AUTH0_AUDIENCE").expect("AUTH0_AUDIENCE must be set");
    let cookie_secret =
        std::env::var("AUTH_COOKIE_SECRET").expect("AUTH_COOKIE_SECRET must be set");
    let internal_token =
        std::env::var("INTERNAL_SERVICE_TOKEN").expect("INTERNAL_SERVICE_TOKEN must be set");
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a valid number");

    let pool = aura_storage_db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool");

    tracing::info!("Database connected and migrations applied");

    let (events_tx, _) = tokio::sync::broadcast::channel::<String>(256);

    let state = AppState {
        pool,
        validator: TokenValidator::new(auth0_domain, auth0_audience, cookie_secret),
        internal_token: InternalToken(internal_token),
        events_tx,
    };

    let cors = match std::env::var("CORS_ORIGINS") {
        Ok(origins) => {
            let allowed: Vec<_> = origins
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            tracing::info!(origins = ?allowed, "CORS restricted to specified origins");
            CorsLayer::new()
                .allow_origin(allowed)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        }
        Err(_) => {
            tracing::warn!("CORS_ORIGINS not set — allowing all origins (development mode)");
            CorsLayer::permissive()
        }
    };

    let app = router::create_router()
        .with_state(state)
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(tower::limit::ConcurrencyLimitLayer::new(512))
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(%addr, "Server starting");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    ctrl_c.await.expect("Failed to listen for shutdown signal");

    tracing::info!("Shutdown signal received");
}
