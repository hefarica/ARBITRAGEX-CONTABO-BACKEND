use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ethers::types::{Address, U256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub rpc_url: String,
    pub ws_url: String,
    pub native_token: String,
    pub gas_config: GasConfig,
    pub dex_configs: HashMap<String, DexConfig>,
    pub flash_loan_providers: Vec<FlashLoanProvider>,
    pub block_time_ms: u64,
    pub confirmation_blocks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasConfig {
    pub base_fee_multiplier: f64,
    pub priority_fee_gwei: u64,
    pub max_gas_price_gwei: u64,
    pub gas_limit_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    pub name: String,
    pub router_address: Address,
    pub factory_address: Address,
    pub fee_bps: u16,
    pub supports_exact_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanProvider {
    pub name: String,
    pub address: Address,
    pub fee_bps: u16,
    pub max_amount: U256,
}

impl ChainConfig {
    pub fn ethereum_mainnet() -> Self {
        let mut dex_configs = HashMap::new();
        
        dex_configs.insert("uniswap_v2".to_string(), DexConfig {
            name: "Uniswap V2".to_string(),
            router_address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap(),
            factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse().unwrap(),
            fee_bps: 30,
            supports_exact_output: true,
        });
        
        dex_configs.insert("uniswap_v3".to_string(), DexConfig {
            name: "Uniswap V3".to_string(),
            router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse().unwrap(),
            factory_address: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse().unwrap(),
            fee_bps: 30,
            supports_exact_output: true,
        });
        
        dex_configs.insert("sushiswap".to_string(), DexConfig {
            name: "SushiSwap".to_string(),
            router_address: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".parse().unwrap(),
            factory_address: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".parse().unwrap(),
            fee_bps: 30,
            supports_exact_output: true,
        });

        Self {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            ws_url: "ws://localhost:8546".to_string(),
            native_token: "ETH".to_string(),
            gas_config: GasConfig {
                base_fee_multiplier: 1.2,
                priority_fee_gwei: 2,
                max_gas_price_gwei: 100,
                gas_limit_multiplier: 1.1,
            },
            dex_configs,
            flash_loan_providers: vec![
                FlashLoanProvider {
                    name: "Aave V3".to_string(),
                    address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".parse().unwrap(),
                    fee_bps: 9,
                    max_amount: U256::from_dec_str("1000000000000000000000").unwrap(),
                },
                FlashLoanProvider {
                    name: "Balancer".to_string(),
                    address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(),
                    fee_bps: 0,
                    max_amount: U256::from_dec_str("10000000000000000000000").unwrap(),
                },
            ],
            block_time_ms: 12000,
            confirmation_blocks: 1,
        }
    }

    pub fn polygon_mainnet() -> Self {
        let mut dex_configs = HashMap::new();
        
        dex_configs.insert("quickswap".to_string(), DexConfig {
            name: "QuickSwap".to_string(),
            router_address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse().unwrap(),
            factory_address: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".parse().unwrap(),
            fee_bps: 30,
            supports_exact_output: true,
        });
        
        dex_configs.insert("sushiswap_polygon".to_string(), DexConfig {
            name: "SushiSwap Polygon".to_string(),
            router_address: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse().unwrap(),
            factory_address: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse().unwrap(),
            fee_bps: 30,
            supports_exact_output: true,
        });

        Self {
            chain_id: 137,
            name: "Polygon Mainnet".to_string(),
            rpc_url: "https://polygon-rpc.com".to_string(),
            ws_url: "wss://polygon-rpc.com".to_string(),
            native_token: "MATIC".to_string(),
            gas_config: GasConfig {
                base_fee_multiplier: 1.1,
                priority_fee_gwei: 30,
                max_gas_price_gwei: 500,
                gas_limit_multiplier: 1.1,
            },
            dex_configs,
            flash_loan_providers: vec![
                FlashLoanProvider {
                    name: "Aave V3 Polygon".to_string(),
                    address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse().unwrap(),
                    fee_bps: 9,
                    max_amount: U256::from_dec_str("1000000000000000000000").unwrap(),
                },
            ],
            block_time_ms: 2000,
            confirmation_blocks: 3,
        }
    }
}

pub struct ChainManager {
    chains: HashMap<u64, ChainConfig>,
}

impl ChainManager {
    pub fn new() -> Self {
        let mut chains = HashMap::new();
        chains.insert(1, ChainConfig::ethereum_mainnet());
        chains.insert(137, ChainConfig::polygon_mainnet());
        
        Self { chains }
    }
    
    pub fn get_chain(&self, chain_id: u64) -> Option<&ChainConfig> {
        self.chains.get(&chain_id)
    }
    
    pub fn get_all_chains(&self) -> &HashMap<u64, ChainConfig> {
        &self.chains
    }
    
    pub fn add_chain(&mut self, config: ChainConfig) {
        self.chains.insert(config.chain_id, config);
    }
}
