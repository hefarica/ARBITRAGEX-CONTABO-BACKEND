use anyhow::Result;
use axum::{
    extract::ws::WebSocketUpgrade,
    response::Response,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber;

mod config;
mod handlers;
mod middleware;
mod models;
mod routes;
mod services;
mod state;
mod ws;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter("selector_api=debug,axum=info")
        .json()
        .init();

    info!("🚀 Starting ArbitrageX Selector API v3.0");

    // Load configuration
    let config = Config::from_env().await?;

    // Initialize database pool
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    // Initialize Redis
    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis_conn = Arc::new(tokio::sync::RwLock::new(
        redis_client.get_tokio_connection().await?,
    ));

    // Create app state
    let app_state = Arc::new(AppState::new(config.clone(), db_pool, redis_conn));

    // Build application
    let app = create_app(app_state).await?;

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🌐 Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(state: Arc<AppState>) -> Result<Router> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))
        
        // API routes
        .nest("/api/v1", routes::api_routes())
        
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        
        // Metrics endpoint
        .route("/metrics", get(handlers::metrics::metrics_handler))
        
        // Add middleware
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    Ok(app)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state))
}

