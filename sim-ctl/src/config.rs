use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub anvil_path: String,
    pub fork_url: String,
    pub fork_block_number: Option<u64>,
    pub fork_timeout: u64,
    pub max_forks: usize,
    pub simulation_gas_limit: u64,
    pub auto_impersonate: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()?,
            anvil_path: std::env::var("ANVIL_PATH")
                .unwrap_or_else(|_| "anvil".to_string()),
            fork_url: std::env::var("FORK_URL")
                .unwrap_or_else(|_| "http://geth-node:8545".to_string()),
            fork_block_number: std::env::var("FORK_BLOCK_NUMBER")
                .ok()
                .and_then(|s| s.parse().ok()),
            fork_timeout: std::env::var("FORK_TIMEOUT")
                .unwrap_or_else(|_| "300".to_string())
                .parse()?,
            max_forks: std::env::var("MAX_FORKS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
            simulation_gas_limit: std::env::var("SIMULATION_GAS_LIMIT")
                .unwrap_or_else(|_| "30000000".to_string())
                .parse()?,
            auto_impersonate: std::env::var("AUTO_IMPERSONATE")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
        })
    }
}



