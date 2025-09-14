use chrono::{DateTime, Utc};
use ethers::types::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fork {
    pub id: Uuid,
    pub rpc_url: String,
    pub block_number: u64,
    pub chain_id: u64,
    pub status: ForkStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForkStatus {
    Active,
    Destroyed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateForkRequest {
    pub block_number: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateRequest {
    pub fork_id: Option<Uuid>,
    pub from: Address,
    pub to: Address,
    pub data: Bytes,
    pub value: U256,
    pub gas: Option<U256>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateResponse {
    pub success: bool,
    pub result: Option<Bytes>,
    pub gas_used: U256,
    pub logs: Vec<Log>,
    pub traces: Vec<Trace>,
    pub revert_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateRequest {
    pub opportunity_id: Uuid,
    pub strategy_type: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub profit: U256,
    pub gas_estimate: U256,
    pub confidence: f64,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IssueSeverity {
    Warning,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<String>,
    pub data: Bytes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Trace {
    pub action: String,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub gas: U256,
    pub input: Bytes,
    pub output: Option<Bytes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub active_forks: usize,
    pub anvil_available: bool,
}



