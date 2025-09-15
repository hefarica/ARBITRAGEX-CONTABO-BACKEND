// ArbitrageX Supreme V3.0 - Shared Types
// Common data structures and types used across all services

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Core business types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChainType {
    Ethereum,
    Polygon,
    Arbitrum,
    Optimism,
    Base,
    BSC,
}

impl std::fmt::Display for ChainType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainType::Ethereum => write!(f, "ethereum"),
            ChainType::Polygon => write!(f, "polygon"),
            ChainType::Arbitrum => write!(f, "arbitrum"),
            ChainType::Optimism => write!(f, "optimism"),
            ChainType::Base => write!(f, "base"),
            ChainType::BSC => write!(f, "bsc"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpportunityStatus {
    Pending,
    Analyzing,
    Selected,
    Executing,
    Completed,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
    Reverted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: Uuid,
    pub chain: ChainType,
    pub token_a: String,
    pub token_b: String,
    pub dex_a: String,
    pub dex_b: String,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
    pub profit_usd: Decimal,
    pub gas_estimate: u64,
    pub gas_price_gwei: u64,
    pub price_impact: Decimal,
    pub confidence_score: Decimal,
    pub status: OpportunityStatus,
    pub block_number: Option<u64>,
    pub transaction_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub id: Uuid,
    pub opportunity_id: Uuid,
    pub tx_hash: Option<String>,
    pub status: ExecutionStatus,
    pub profit_expected_usd: Option<Decimal>,
    pub profit_actual_usd: Option<Decimal>,
    pub gas_estimate: Option<u64>,
    pub gas_used: Option<u64>,
    pub gas_price_estimate_gwei: Option<u64>,
    pub gas_price_actual_gwei: Option<u64>,
    pub block_number: Option<u64>,
    pub executed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub metadata: HashMap<String, serde_json::Value>,
}

// API response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub request_id: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
            request_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: Utc::now(),
            request_id: Uuid::new_v4().to_string(),
        }
    }
}

// Configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub password: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub database: u8,
    pub max_connections: u32,
    pub connection_timeout: u64,
    pub command_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub ethereum_rpc: String,
    pub ethereum_ws: String,
    pub polygon_rpc: String,
    pub arbitrum_rpc: String,
    pub optimism_rpc: String,
    pub base_rpc: String,
    pub bsc_rpc: String,
}

// Metrics types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub total_opportunities: u64,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub total_profit_usd: Decimal,
    pub total_gas_cost_usd: Decimal,
    pub net_profit_usd: Decimal,
    pub success_rate: f64,
    pub avg_profit_per_execution: Decimal,
    pub uptime_seconds: u64,
    pub last_opportunity: Option<DateTime<Utc>>,
    pub last_execution: Option<DateTime<Utc>>,
}

// Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    OpportunityDetected(Uuid),
    OpportunitySelected(Uuid),
    ExecutionStarted(Uuid),
    ExecutionCompleted(Uuid),
    ExecutionFailed(Uuid, String),
    BundleSubmitted(String, String), // bundle_id, relay_name
    ReconciliationCompleted(Uuid),
    SystemAlert(String, AlertLevel),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

// Query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityQuery {
    pub chain: Option<ChainType>,
    pub status: Option<OpportunityStatus>,
    pub min_profit: Option<Decimal>,
    pub max_profit: Option<Decimal>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionQuery {
    pub opportunity_id: Option<Uuid>,
    pub status: Option<ExecutionStatus>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

// Health check types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub services: HashMap<String, ServiceHealth>,
    pub database_connected: bool,
    pub redis_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub response_time_ms: Option<f64>,
    pub error_rate: f64,
}

// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub user_id: String,
    pub role: String,
}

// MEV and Bundle types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub id: String,
    pub transactions: Vec<String>,
    pub block_number: u64,
    pub min_timestamp: Option<u64>,
    pub max_timestamp: Option<u64>,
    pub replacement_uuid: Option<String>,
    pub signing_address: Option<String>,
    pub refund_percent: Option<u8>,
    pub refund_recipient: Option<String>,
    pub refund_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySubmission {
    pub bundle: Bundle,
    pub relay: String,
    pub submission_time: DateTime<Utc>,
    pub status: SubmissionStatus,
    pub response: Option<RelayResponse>,
    pub error: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubmissionStatus {
    Pending,
    Submitted,
    Accepted,
    Rejected,
    Included,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayResponse {
    pub bundle_hash: Option<String>,
    pub simulation_success: Option<bool>,
    pub simulation_error: Option<String>,
    pub coinbase_diff: Option<String>,
    pub eth_sent_to_coinbase: Option<String>,
    pub gas_fees: Option<String>,
    pub gas_used: Option<u64>,
    pub logs: Option<Vec<serde_json::Value>>,
}

// Utility functions for type conversions
impl From<String> for ChainType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "ethereum" => ChainType::Ethereum,
            "polygon" => ChainType::Polygon,
            "arbitrum" => ChainType::Arbitrum,
            "optimism" => ChainType::Optimism,
            "base" => ChainType::Base,
            "bsc" => ChainType::BSC,
            _ => ChainType::Ethereum, // Default fallback
        }
    }
}

impl From<&str> for ChainType {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl From<String> for OpportunityStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => OpportunityStatus::Pending,
            "analyzing" => OpportunityStatus::Analyzing,
            "selected" => OpportunityStatus::Selected,
            "executing" => OpportunityStatus::Executing,
            "completed" => OpportunityStatus::Completed,
            "failed" => OpportunityStatus::Failed,
            "expired" => OpportunityStatus::Expired,
            _ => OpportunityStatus::Pending, // Default fallback
        }
    }
}

impl From<String> for ExecutionStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => ExecutionStatus::Pending,
            "submitted" => ExecutionStatus::Submitted,
            "confirmed" => ExecutionStatus::Confirmed,
            "failed" => ExecutionStatus::Failed,
            "reverted" => ExecutionStatus::Reverted,
            _ => ExecutionStatus::Pending, // Default fallback
        }
    }
}

// Constants
pub const DEFAULT_PAGE_SIZE: u32 = 50;
pub const MAX_PAGE_SIZE: u32 = 1000;
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
pub const MIN_PROFIT_USD: f64 = 1.0;
pub const MAX_GAS_PRICE_GWEI: u64 = 500;
pub const MAX_SLIPPAGE_PERCENT: f64 = 5.0;

// Test utilities
#[cfg(test)]
pub mod test_utils {
    use super::*;

    pub fn create_test_opportunity() -> Opportunity {
        Opportunity {
            id: Uuid::new_v4(),
            chain: ChainType::Ethereum,
            token_a: "0xA0b86a33E6Ba3E6F1b5F6B5A5C5B5D5E5F5G5H5I".to_string(),
            token_b: "0xB0b86a33E6Ba3E6F1b5F6B5A5C5B5D5E5F5G5H5I".to_string(),
            dex_a: "Uniswap V3".to_string(),
            dex_b: "SushiSwap".to_string(),
            amount_in: Decimal::from(1000),
            amount_out: Decimal::from(1050),
            profit_usd: Decimal::from(50),
            gas_estimate: 150000,
            gas_price_gwei: 20,
            price_impact: Decimal::from_f64(0.5).unwrap(),
            confidence_score: Decimal::from_f64(0.95).unwrap(),
            status: OpportunityStatus::Pending,
            block_number: Some(18500000),
            transaction_hash: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::minutes(5)),
            metadata: HashMap::new(),
        }
    }

    pub fn create_test_execution(opportunity_id: Uuid) -> Execution {
        Execution {
            id: Uuid::new_v4(),
            opportunity_id,
            tx_hash: Some("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string()),
            status: ExecutionStatus::Confirmed,
            profit_expected_usd: Some(Decimal::from(50)),
            profit_actual_usd: Some(Decimal::from(48)),
            gas_estimate: Some(150000),
            gas_used: Some(145000),
            gas_price_estimate_gwei: Some(20),
            gas_price_actual_gwei: Some(22),
            block_number: Some(18500001),
            executed_at: Utc::now(),
            confirmed_at: Some(Utc::now() + chrono::Duration::seconds(15)),
            error_message: None,
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }
}
