use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Opportunity {
    pub id: Uuid,
    pub chain: String,
    pub strategy_type: String,
    pub tokens: Vec<String>,
    pub pools: Vec<String>,
    pub expected_profit: i64,
    pub gas_cost: i64,
    pub confidence: f64,
    pub deadline: i64,
    pub status: OpportunityStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "opportunity_status", rename_all = "snake_case")]
pub enum OpportunityStatus {
    Pending,
    Simulating,
    Approved,
    Executing,
    Completed,
    Failed,
    Expired,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Execution {
    pub id: Uuid,
    pub opportunity_id: Uuid,
    pub tx_hash: Option<String>,
    pub status: ExecutionStatus,
    pub actual_profit: Option<i64>,
    pub gas_used: Option<i64>,
    pub error_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "execution_status", rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
    Reverted,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateOpportunityRequest {
    #[validate(length(min = 1, max = 50))]
    pub chain: String,
    #[validate(length(min = 1, max = 50))]
    pub strategy_type: String,
    #[validate(length(min = 1))]
    pub tokens: Vec<String>,
    #[validate(length(min = 1))]
    pub pools: Vec<String>,
    #[validate(range(min = 0))]
    pub expected_profit: i64,
    #[validate(range(min = 0))]
    pub gas_cost: i64,
    #[validate(range(min = 0.0, max = 1.0))]
    pub confidence: f64,
    pub deadline: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpportunityResponse {
    pub opportunity: Opportunity,
    pub can_execute: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PaginationQuery {
    #[validate(range(min = 1))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub opportunity_id: Uuid,
    pub gas_price: Option<i64>,
    pub gas_limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub opportunity_id: Uuid,
    pub fork_block: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulationResponse {
    pub success: bool,
    pub profit: i64,
    pub gas_used: i64,
    pub logs: Vec<String>,
    pub revert_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: bool,
    pub redis: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub total_opportunities: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub total_profit: i64,
    pub avg_gas_cost: f64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub event_type: WsEventType,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsEventType {
    OpportunityNew,
    OpportunityUpdate,
    ExecutionStart,
    ExecutionComplete,
    MarketUpdate,
    Heartbeat,
}



