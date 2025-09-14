use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;

use crate::models::HealthResponse;
use crate::state::AppState;

pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    // Check database connection
    let db_ok = sqlx::query!("SELECT 1 as one")
        .fetch_one(&state.db_pool)
        .await
        .is_ok();

    // Check Redis connection
    let redis_ok = {
        let mut conn = state.redis_conn.write().await;
        redis::cmd("PING")
            .query_async::<_, String>(&mut *conn)
            .await
            .is_ok()
    };

    Json(HealthResponse {
        status: if db_ok && redis_ok { "healthy" } else { "unhealthy" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_ok,
        redis: redis_ok,
        timestamp: Utc::now(),
    })
}



