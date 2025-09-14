use axum::{extract::State, Json};
use std::sync::Arc;

use crate::models::MetricsResponse;
use crate::state::AppState;

pub async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> Json<MetricsResponse> {
    // Get metrics from database
    let metrics = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total_opportunities,
            COUNT(*) FILTER (WHERE status = 'completed') as successful_executions,
            COUNT(*) FILTER (WHERE status = 'failed') as failed_executions,
            COALESCE(SUM(actual_profit), 0) as total_profit,
            COALESCE(AVG(gas_used), 0) as avg_gas_cost
        FROM executions e
        JOIN opportunities o ON e.opportunity_id = o.id
        "#
    )
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or_default();

    let success_rate = if metrics.total_opportunities.unwrap_or(0) > 0 {
        metrics.successful_executions.unwrap_or(0) as f64 / 
        metrics.total_opportunities.unwrap_or(1) as f64 * 100.0
    } else {
        0.0
    };

    Json(MetricsResponse {
        total_opportunities: metrics.total_opportunities.unwrap_or(0),
        successful_executions: metrics.successful_executions.unwrap_or(0),
        failed_executions: metrics.failed_executions.unwrap_or(0),
        total_profit: metrics.total_profit.unwrap_or(0),
        avg_gas_cost: metrics.avg_gas_cost.unwrap_or(0.0),
        success_rate,
    })
}



