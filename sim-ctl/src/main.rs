use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber;

mod anvil;
mod config;
mod handlers;
mod models;
mod validation;

use crate::anvil::ForkManager;
use crate::config::Config;

pub struct AppState {
    pub config: Config,
    pub fork_manager: Arc<RwLock<ForkManager>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("sim_ctl=debug,ethers=info")
        .json()
        .init();

    info!("🔧 Starting ArbitrageX Sim-CTL v3.0");

    // Load configuration
    let config = Config::from_env()?;

    // Initialize fork manager
    let fork_manager = Arc::new(RwLock::new(ForkManager::new(config.clone())));

    // Create app state
    let state = Arc::new(AppState {
        config: config.clone(),
        fork_manager,
    });

    // Build routes
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/fork/create", post(handlers::create_fork))
        .route("/fork/:id", get(handlers::get_fork_info))
        .route("/fork/:id/destroy", post(handlers::destroy_fork))
        .route("/simulate", post(handlers::simulate_transaction))
        .route("/validate", post(handlers::validate_opportunity))
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🌐 Sim-CTL listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}



