use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;

use crate::handlers::{executions, opportunities, simulation};
use crate::middleware::auth;
use crate::state::AppState;

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Public routes
        .nest("/public", public_routes())
        // Protected routes
        .nest("/", protected_routes())
        .layer(axum::middleware::from_fn(auth::auth_middleware))
}

fn public_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/opportunities", get(opportunities::list_opportunities))
        .route("/opportunities/:id", get(opportunities::get_opportunity))
        .route("/executions/:id", get(executions::get_execution))
}

fn protected_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Opportunities
        .route("/opportunities", post(opportunities::create_opportunity))
        .route("/opportunities/:id", put(opportunities::update_opportunity))
        
        // Simulations
        .route("/simulate", post(simulation::simulate_opportunity))
        
        // Executions
        .route("/execute", post(executions::execute_opportunity))
        .route("/executions", get(executions::list_executions))
}



