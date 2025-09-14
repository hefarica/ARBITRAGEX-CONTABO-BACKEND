use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::models::{SimulationRequest, SimulationResponse};
use crate::state::AppState;

pub async fn simulate_opportunity(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SimulationRequest>,
) -> Result<Json<SimulationResponse>, StatusCode> {
    // Verify opportunity exists
    let opportunity = sqlx::query!(
        "SELECT expected_profit, gas_cost FROM opportunities WHERE id = $1",
        request.opportunity_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // TODO: Call sim-ctl service for actual simulation
    // For now, return mock simulation
    let response = SimulationResponse {
        success: true,
        profit: opportunity.expected_profit,
        gas_used: opportunity.gas_cost,
        logs: vec![
            format!("Fork created at block {}", request.fork_block.unwrap_or(0)),
            "Simulation started".to_string(),
            "Transaction executed successfully".to_string(),
            format!("Profit: {} wei", opportunity.expected_profit),
        ],
        revert_reason: None,
    };

    Ok(Json(response))
}



