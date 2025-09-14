use chrono::{DateTime, Utc};
use ethers::types::{Address, Transaction, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    pub ws_url: String,
    pub redis_url: String,
    pub database_url: String,
    pub block_time: u64,
    pub min_profit_wei: u64,
    pub max_gas_price: U256,
    pub flashloan_providers: Vec<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config {
            rpc_url: std::env::var("RPC_URL")
                .unwrap_or_else(|_| "http://geth-node:8545".to_string()),
            ws_url: std::env::var("WS_URL")
                .unwrap_or_else(|_| "ws://geth-node:8546".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://redis-cache:6379".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://arbitragex:password@postgres-db:5432/arbitragex".to_string()),
            block_time: 12000, // 12 segundos
            min_profit_wei: 10_000_000_000_000_000, // 0.01 ETH
            max_gas_price: U256::from(100_000_000_000u64), // 100 gwei
            flashloan_providers: vec![
                "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(), // Balancer
                "0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9".to_string(), // Aave V2
            ],
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: String,
    pub chain: String,
    pub strategy_type: String,
    pub tokens: Vec<String>,
    pub pools: Vec<String>,
    pub expected_profit: U256,
    pub gas_cost: U256,
    pub confidence: f64,
    pub deadline: u64,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct MarketState {
    pub block_number: u64,
    pub block_timestamp: u64,
    pub gas_price: U256,
    pub token_prices: HashMap<String, TokenPrice>,
    pub pools: HashMap<String, PoolState>,
    pub recent_transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub symbol: String,
    pub price_usd: f64,
    pub last_update: u64,
}

#[derive(Debug, Clone)]
pub struct PoolState {
    pub address: String,
    pub token0: String,
    pub token1: String,
    pub reserve0: U256,
    pub reserve1: U256,
    pub fee: u32,
    pub last_update: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageParams {
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub pools: Vec<PoolInfo>,
    pub min_profit: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: Address,
    pub pool_type: PoolType,
    pub fee: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    Balancer,
    Curve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub profit: U256,
    pub gas_used: U256,
    pub revert_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub opportunity_id: String,
    pub tx_hash: String,
    pub success: bool,
    pub actual_profit: U256,
    pub gas_used: U256,
    pub executed_at: DateTime<Utc>,
}

