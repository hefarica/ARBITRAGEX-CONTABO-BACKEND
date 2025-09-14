use chrono::{DateTime, Utc};
use ethers::types::{Address, Transaction, U256, H256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub chains: HashMap<u64, ChainConfig>,
    pub strategies: Vec<StrategyConfig>,
    pub redis_url: String,
    pub database_url: String,
    pub api_port: u16,
    pub min_profit_wei: u64,
    pub max_gas_price: U256,
    pub flashloan_providers: HashMap<u64, Vec<String>>,
    pub coingecko_api_key: Option<String>,
    pub defi_llama_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub rpc_url: String,
    pub ws_url: String,
    pub block_time: u64,
    pub gas_limit: u64,
    pub base_fee_multiplier: f64,
    pub priority_fee: U256,
    pub native_token: String,
    pub wrapped_native: String,
    pub dex_routers: HashMap<String, String>,
    pub flashloan_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub name: String,
    pub enabled: bool,
    pub min_profit_wei: u64,
    pub max_gas_price: U256,
    pub max_slippage: f64,
    pub timeout_seconds: u64,
    pub parameters: HashMap<String, serde_json::Value>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let mut chains = HashMap::new();
        
        // Ethereum Mainnet
        chains.insert(1, ChainConfig {
            chain_id: 1,
            name: "Ethereum".to_string(),
            rpc_url: std::env::var("ETH_RPC_URL")
                .unwrap_or_else(|_| "http://geth-node:8545".to_string()),
            ws_url: std::env::var("ETH_WS_URL")
                .unwrap_or_else(|_| "ws://geth-node:8546".to_string()),
            block_time: 12000,
            gas_limit: 30_000_000,
            base_fee_multiplier: 1.1,
            priority_fee: U256::from(2_000_000_000u64), // 2 gwei
            native_token: "ETH".to_string(),
            wrapped_native: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            dex_routers: [
                ("uniswap_v2".to_string(), "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string()),
                ("uniswap_v3".to_string(), "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                ("sushiswap".to_string(), "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string()),
            ].into_iter().collect(),
            flashloan_providers: vec![
                "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(), // Balancer
                "0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9".to_string(), // Aave V2
            ],
        });
        
        // Polygon
        chains.insert(137, ChainConfig {
            chain_id: 137,
            name: "Polygon".to_string(),
            rpc_url: std::env::var("POLYGON_RPC_URL")
                .unwrap_or_else(|_| "https://polygon-rpc.com".to_string()),
            ws_url: std::env::var("POLYGON_WS_URL")
                .unwrap_or_else(|_| "wss://polygon-rpc.com/ws".to_string()),
            block_time: 2000,
            gas_limit: 20_000_000,
            base_fee_multiplier: 1.2,
            priority_fee: U256::from(30_000_000_000u64), // 30 gwei
            native_token: "MATIC".to_string(),
            wrapped_native: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".to_string(),
            dex_routers: [
                ("quickswap".to_string(), "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".to_string()),
                ("sushiswap".to_string(), "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".to_string()),
            ].into_iter().collect(),
            flashloan_providers: vec![
                "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(), // Balancer
            ],
        });
        
        let strategies = vec![
            StrategyConfig {
                name: "triangular_arbitrage".to_string(),
                enabled: true,
                min_profit_wei: 10_000_000_000_000_000, // 0.01 ETH
                max_gas_price: U256::from(100_000_000_000u64), // 100 gwei
                max_slippage: 0.005, // 0.5%
                timeout_seconds: 30,
                parameters: HashMap::new(),
            },
            StrategyConfig {
                name: "cross_dex_arbitrage".to_string(),
                enabled: true,
                min_profit_wei: 5_000_000_000_000_000, // 0.005 ETH
                max_gas_price: U256::from(80_000_000_000u64), // 80 gwei
                max_slippage: 0.01, // 1%
                timeout_seconds: 45,
                parameters: HashMap::new(),
            },
        ];
        
        Ok(Config {
            chains,
            strategies,
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://redis-cache:6379".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://arbitragex:password@postgres-db:5432/arbitragex".to_string()),
            api_port: std::env::var("API_PORT")
                .unwrap_or_else(|_| "8079".to_string())
                .parse()
                .unwrap_or(8079),
            min_profit_wei: 10_000_000_000_000_000, // 0.01 ETH
            max_gas_price: U256::from(100_000_000_000u64), // 100 gwei
            flashloan_providers: HashMap::new(),
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
            defi_llama_api_key: std::env::var("DEFI_LLAMA_API_KEY").ok(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: String,
    pub chain_id: u64,
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
    pub token_id: String,
    pub price_usd: f64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexPool {
    pub address: String,
    pub chain: String,
    pub project: String,
    pub symbol: String,
    pub tvl_usd: f64,
    pub apy: f64,
    pub volume_usd_1d: f64,
    pub volume_usd_7d: f64,
    pub last_updated: u64,
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

