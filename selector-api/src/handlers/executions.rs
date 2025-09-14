use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{ExecuteRequest, Execution, ExecutionStatus};
use crate::state::AppState;

pub async fn execute_opportunity(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExecuteRequest>,
) -> Result<Json<Execution>, StatusCode> {
    // Verify opportunity exists and can be executed
    let opportunity = sqlx::query!(
        "SELECT id, status FROM opportunities WHERE id = $1",
        request.opportunity_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Create execution record
    let execution = sqlx::query_as!(
        Execution,
        r#"
        INSERT INTO executions (
            id, opportunity_id, status, created_at
        )
        VALUES ($1, $2, $3, $4)
        RETURNING 
            id, opportunity_id, tx_hash, 
            status as "status: ExecutionStatus",
            actual_profit, gas_used, error_reason,
            created_at, completed_at
        "#,
        Uuid::new_v4(),
        request.opportunity_id,
        ExecutionStatus::Pending as ExecutionStatus,
        chrono::Utc::now()
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Publish execution start event
    let mut conn = state.redis_conn.write().await;
    let _ = redis::cmd("PUBLISH")
        .arg("executions")
        .arg(serde_json::to_string(&execution).unwrap())
        .query_async::<_, ()>(&mut *conn)
        .await;

    // TODO: Trigger actual execution via sim-ctl

    Ok(Json(execution))
}

pub async fn get_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Execution>, StatusCode> {
    let execution = sqlx::query_as!(
        Execution,
        r#"
        SELECT 
            id, opportunity_id, tx_hash,
            status as "status: ExecutionStatus",
            actual_profit, gas_used, error_reason,
            created_at, completed_at
        FROM executions
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(execution))
}

pub async fn list_executions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Execution>>, StatusCode> {
    let executions = sqlx::query_as!(
        Execution,
        r#"
        SELECT 
            id, opportunity_id, tx_hash,
            status as "status: ExecutionStatus",
            actual_profit, gas_used, error_reason,
            created_at, completed_at
        FROM executions
        ORDER BY created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(executions))
}



