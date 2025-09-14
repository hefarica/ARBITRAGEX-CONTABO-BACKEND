use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use ethers::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::*;
use crate::validation::validator::validate_opportunity;
use crate::AppState;

pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let fork_manager = state.fork_manager.read().await;
    let active_forks = fork_manager.list_forks().len();
    
    // Check if Anvil is available
    let anvil_available = std::process::Command::new(&state.config.anvil_path)
        .arg("--version")
        .output()
        .is_ok();

    Json(HealthResponse {
        status: "healthy".to_string(),
        active_forks,
        anvil_available,
    })
}

pub async fn create_fork(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateForkRequest>,
) -> Result<Json<Fork>, StatusCode> {
    let mut fork_manager = state.fork_manager.write().await;
    
    match fork_manager.create_fork(request.block_number).await {
        Ok(fork) => Ok(Json(fork)),
        Err(e) => {
            tracing::error!("Failed to create fork: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_fork_info(
    State(state): State<Arc<AppState>>,
    Path(fork_id): Path<Uuid>,
) -> Result<Json<Fork>, StatusCode> {
    let fork_manager = state.fork_manager.read().await;
    
    match fork_manager.get_fork(&fork_id) {
        Some(fork) => Ok(Json(fork.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn destroy_fork(
    State(state): State<Arc<AppState>>,
    Path(fork_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let mut fork_manager = state.fork_manager.write().await;
    
    match fork_manager.destroy_fork(fork_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn simulate_transaction(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, StatusCode> {
    let fork_manager = state.fork_manager.read().await;
    
    // Get or create fork
    let fork_id = if let Some(id) = request.fork_id {
        id
    } else {
        // Create temporary fork for simulation
        drop(fork_manager);
        let mut fork_manager = state.fork_manager.write().await;
        let fork = fork_manager.create_fork(None).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        fork.id
    };

    let fork_manager = state.fork_manager.read().await;
    let provider = fork_manager.get_provider(&fork_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Enable auto-impersonation if configured
    if state.config.auto_impersonate {
        let _ = provider
            .request::<_, ()>("anvil_impersonateAccount", vec![request.from])
            .await;
    }

    // Prepare transaction
    let tx = TransactionRequest::new()
        .from(request.from)
        .to(request.to)
        .data(request.data.clone())
        .value(request.value)
        .gas(request.gas.unwrap_or(U256::from(state.config.simulation_gas_limit)));

    // Execute call
    let result = provider.call(&tx.into(), None).await;

    // Get traces
    let traces_result = provider
        .request::<_, Vec<serde_json::Value>>("trace_call", vec![
            serde_json::to_value(&tx).unwrap(),
            serde_json::json!(["trace"]),
            serde_json::json!("latest"),
        ])
        .await;

    let (success, result_data, revert_reason) = match result {
        Ok(data) => (true, Some(data), None),
        Err(e) => {
            let reason = e.to_string();
            (false, None, Some(reason))
        }
    };

    // Estimate gas if successful
    let gas_used = if success {
        provider.estimate_gas(&tx.into(), None).await.unwrap_or_default()
    } else {
        U256::zero()
    };

    Ok(Json(SimulateResponse {
        success,
        result: result_data,
        gas_used,
        logs: vec![], // TODO: Extract logs from trace
        traces: vec![], // TODO: Parse traces
        revert_reason,
    }))
}

pub async fn validate_opportunity(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, StatusCode> {
    // Create fork for validation
    let mut fork_manager = state.fork_manager.write().await;
    let fork = fork_manager.create_fork(None).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    drop(fork_manager);
    
    let fork_manager = state.fork_manager.read().await;
    let provider = fork_manager.get_provider(&fork.id).unwrap();
    
    // Run validation
    let response = validate_opportunity(
        &request.opportunity_id,
        &request.strategy_type,
        &request.params,
        provider.as_ref(),
    ).await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Clean up fork
    drop(fork_manager);
    let mut fork_manager = state.fork_manager.write().await;
    let _ = fork_manager.destroy_fork(fork.id).await;
    
    Ok(Json(response))
}



