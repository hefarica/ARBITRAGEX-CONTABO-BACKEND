use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{
    CreateOpportunityRequest, Opportunity, OpportunityResponse, OpportunityStatus,
    PaginatedResponse,
};
use crate::state::AppState;

pub async fn list_opportunities(
    state: &Arc<AppState>,
    page: i64,
    per_page: i64,
) -> Result<PaginatedResponse<Opportunity>> {
    let offset = (page - 1) * per_page;

    // Get total count
    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM opportunities WHERE status != 'expired'"
    )
    .fetch_one(&state.db_pool)
    .await?
    .unwrap_or(0);

    // Get opportunities
    let opportunities = sqlx::query_as!(
        Opportunity,
        r#"
        SELECT 
            id, chain, strategy_type, tokens, pools,
            expected_profit, gas_cost, confidence, deadline,
            status as "status: OpportunityStatus",
            created_at, updated_at
        FROM opportunities
        WHERE status != 'expired'
        ORDER BY expected_profit DESC
        LIMIT $1 OFFSET $2
        "#,
        per_page,
        offset
    )
    .fetch_all(&state.db_pool)
    .await?;

    let total_pages = (total + per_page - 1) / per_page;

    Ok(PaginatedResponse {
        data: opportunities,
        total,
        page,
        per_page,
        total_pages,
    })
}

pub async fn get_opportunity(
    state: &Arc<AppState>,
    id: Uuid,
) -> Result<Option<OpportunityResponse>> {
    let opportunity = sqlx::query_as!(
        Opportunity,
        r#"
        SELECT 
            id, chain, strategy_type, tokens, pools,
            expected_profit, gas_cost, confidence, deadline,
            status as "status: OpportunityStatus",
            created_at, updated_at
        FROM opportunities
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db_pool)
    .await?;

    if let Some(opp) = opportunity {
        let can_execute = can_execute_opportunity(&opp);
        Ok(Some(OpportunityResponse {
            opportunity: opp,
            can_execute,
            reason: if !can_execute {
                Some("Insufficient profit or expired".to_string())
            } else {
                None
            },
        }))
    } else {
        Ok(None)
    }
}

pub async fn create_opportunity(
    state: &Arc<AppState>,
    request: CreateOpportunityRequest,
) -> Result<Opportunity> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let opportunity = sqlx::query_as!(
        Opportunity,
        r#"
        INSERT INTO opportunities (
            id, chain, strategy_type, tokens, pools,
            expected_profit, gas_cost, confidence, deadline,
            status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING 
            id, chain, strategy_type, tokens, pools,
            expected_profit, gas_cost, confidence, deadline,
            status as "status: OpportunityStatus",
            created_at, updated_at
        "#,
        id,
        request.chain,
        request.strategy_type,
        &request.tokens,
        &request.pools,
        request.expected_profit,
        request.gas_cost,
        request.confidence,
        request.deadline,
        OpportunityStatus::Pending as OpportunityStatus,
        now,
        now
    )
    .fetch_one(&state.db_pool)
    .await?;

    // Publish to Redis for WebSocket
    let mut conn = state.redis_conn.write().await;
    let _ = redis::cmd("PUBLISH")
        .arg("opportunities")
        .arg(serde_json::to_string(&opportunity)?)
        .query_async::<_, ()>(&mut *conn)
        .await;

    Ok(opportunity)
}

pub async fn update_opportunity_status(
    state: &Arc<AppState>,
    id: Uuid,
    status: OpportunityStatus,
) -> Result<Option<Opportunity>> {
    let opportunity = sqlx::query_as!(
        Opportunity,
        r#"
        UPDATE opportunities
        SET status = $2, updated_at = $3
        WHERE id = $1
        RETURNING 
            id, chain, strategy_type, tokens, pools,
            expected_profit, gas_cost, confidence, deadline,
            status as "status: OpportunityStatus",
            created_at, updated_at
        "#,
        id,
        status as OpportunityStatus,
        Utc::now()
    )
    .fetch_optional(&state.db_pool)
    .await?;

    Ok(opportunity)
}

fn can_execute_opportunity(opp: &Opportunity) -> bool {
    let now = Utc::now().timestamp();
    opp.deadline > now && 
    opp.expected_profit > opp.gas_cost &&
    matches!(opp.status, OpportunityStatus::Approved | OpportunityStatus::Pending)
}



