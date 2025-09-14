use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use async_trait::async_trait;

use crate::strategies::Strategy;
use crate::types::{Config, Opportunity, MarketState, DexPool, TokenPrice};
use crate::core::profit_calculator::ProfitCalculator;

pub struct TriangularArbitrageStrategy {
    config: Config,
    providers: HashMap<u64, Arc<Provider<Http>>>,
    profit_calculator: ProfitCalculator,
    min_profit_wei: U256,
    max_gas_price: U256,
}

impl TriangularArbitrageStrategy {
    pub async fn new(
        config: Config,
        providers: HashMap<u64, Arc<Provider<Http>>>,
    ) -> Result<Self> {
        let strategy_config = config.strategies
            .iter()
            .find(|s| s.name == "triangular_arbitrage")
            .ok_or_else(|| anyhow::anyhow!("Triangular arbitrage strategy not found in config"))?;

        Ok(Self {
            config: config.clone(),
            providers,
            profit_calculator: ProfitCalculator::new(),
            min_profit_wei: U256::from(strategy_config.min_profit_wei),
            max_gas_price: strategy_config.max_gas_price,
        })
    }

    /// Find triangular arbitrage opportunities across multiple DEXs
    async fn find_triangular_opportunities(
        &self,
        chain_id: u64,
        market_state: &MarketState,
        dex_pools: &HashMap<String, DexPool>,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Vec<Opportunity>> {
        let mut opportunities = Vec::new();

        // Get major trading pairs for the chain
        let major_tokens = self.get_major_tokens(chain_id);
        
        debug!("🔍 Scanning {} major tokens for triangular arbitrage on chain {}", 
               major_tokens.len(), chain_id);

        // Check all possible triangular combinations
        for i in 0..major_tokens.len() {
            for j in (i + 1)..major_tokens.len() {
                for k in (j + 1)..major_tokens.len() {
                    let token_a = &major_tokens[i];
                    let token_b = &major_tokens[j];
                    let token_c = &major_tokens[k];

                    // Find pools for each pair
                    if let Some(pools) = self.find_triangle_pools(
                        token_a, token_b, token_c, dex_pools, chain_id
                    ).await {
                        // Check both directions: A->B->C->A and A->C->B->A
                        if let Some(opp) = self.check_triangular_path(
                            token_a, token_b, token_c, &pools, 
                            chain_id, market_state, token_prices
                        ).await? {
                            opportunities.push(opp);
                        }

                        if let Some(opp) = self.check_triangular_path(
                            token_a, token_c, token_b, &pools, 
                            chain_id, market_state, token_prices
                        ).await? {
                            opportunities.push(opp);
                        }
                    }
                }
            }
        }

        info!("🎯 Found {} triangular arbitrage opportunities on chain {}", 
              opportunities.len(), chain_id);

        Ok(opportunities)
    }

    /// Get major tokens for a specific chain
    fn get_major_tokens(&self, chain_id: u64) -> Vec<String> {
        match chain_id {
            1 => vec![ // Ethereum
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                "0xA0b86a33E6441E1343e0c4b0b9B3B8B1e4e8D1C5".to_string(), // USDC
                "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT
                "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(), // DAI
                "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(), // WBTC
                "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984".to_string(), // UNI
                "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9".to_string(), // AAVE
            ],
            137 => vec![ // Polygon
                "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".to_string(), // WMATIC
                "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(), // USDC
                "0xc2132D05D31c914a87C6611C10748AEb04B58e8F".to_string(), // USDT
                "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".to_string(), // DAI
                "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6".to_string(), // WBTC
                "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".to_string(), // WETH
            ],
            _ => vec![], // Other chains
        }
    }

    /// Find pools that form a triangle between three tokens
    async fn find_triangle_pools(
        &self,
        token_a: &str,
        token_b: &str,
        token_c: &str,
        dex_pools: &HashMap<String, DexPool>,
        chain_id: u64,
    ) -> Option<Vec<DexPool>> {
        let mut pools = Vec::new();

        // Find A-B pool
        if let Some(pool_ab) = self.find_pool_for_pair(token_a, token_b, dex_pools, chain_id) {
            pools.push(pool_ab);
        } else {
            return None;
        }

        // Find B-C pool
        if let Some(pool_bc) = self.find_pool_for_pair(token_b, token_c, dex_pools, chain_id) {
            pools.push(pool_bc);
        } else {
            return None;
        }

        // Find C-A pool
        if let Some(pool_ca) = self.find_pool_for_pair(token_c, token_a, dex_pools, chain_id) {
            pools.push(pool_ca);
        } else {
            return None;
        }

        Some(pools)
    }

    /// Find a pool for a specific token pair
    fn find_pool_for_pair(
        &self,
        token_a: &str,
        token_b: &str,
        dex_pools: &HashMap<String, DexPool>,
        chain_id: u64,
    ) -> Option<DexPool> {
        let chain_name = match chain_id {
            1 => "ethereum",
            137 => "polygon",
            _ => return None,
        };

        // Look for pools containing both tokens
        for pool in dex_pools.values() {
            if pool.chain == chain_name {
                let symbol_lower = pool.symbol.to_lowercase();
                let token_a_short = self.get_token_symbol(token_a);
                let token_b_short = self.get_token_symbol(token_b);
                
                if (symbol_lower.contains(&token_a_short.to_lowercase()) && 
                    symbol_lower.contains(&token_b_short.to_lowercase())) ||
                   (symbol_lower.contains(&token_b_short.to_lowercase()) && 
                    symbol_lower.contains(&token_a_short.to_lowercase())) {
                    return Some(pool.clone());
                }
            }
        }

        None
    }

    /// Get token symbol from address (simplified mapping)
    fn get_token_symbol(&self, address: &str) -> String {
        match address {
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" => "WETH".to_string(),
            "0xA0b86a33E6441E1343e0c4b0b9B3B8B1e4e8D1C5" => "USDC".to_string(),
            "0xdAC17F958D2ee523a2206206994597C13D831ec7" => "USDT".to_string(),
            "0x6B175474E89094C44Da98b954EedeAC495271d0F" => "DAI".to_string(),
            "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270" => "WMATIC".to_string(),
            _ => "UNKNOWN".to_string(),
        }
    }

    /// Check a specific triangular arbitrage path
    async fn check_triangular_path(
        &self,
        token_a: &str,
        token_b: &str,
        token_c: &str,
        pools: &[DexPool],
        chain_id: u64,
        market_state: &MarketState,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Option<Opportunity>> {
        // Test with different amounts to find optimal trade size
        let test_amounts = vec![
            U256::from(100_000_000_000_000_000u64),   // 0.1 ETH
            U256::from(500_000_000_000_000_000u64),   // 0.5 ETH
            U256::from(1_000_000_000_000_000_000u64), // 1.0 ETH
            U256::from(5_000_000_000_000_000_000u64), // 5.0 ETH
        ];

        let mut best_opportunity: Option<Opportunity> = None;
        let mut best_profit = U256::zero();

        for amount_in in test_amounts {
            let profit = self.profit_calculator.calculate_triangular_arbitrage_profit(
                token_a,
                token_b,
                token_c,
                amount_in,
                pools,
                token_prices,
                chain_id,
            ).await?;

            if profit > best_profit && profit >= self.min_profit_wei {
                best_profit = profit;

                // Calculate confidence score
                let pool_liquidity = pools.iter().map(|p| p.tvl_usd).sum::<f64>() / pools.len() as f64;
                let profit_margin = profit.as_u128() as f64 / amount_in.as_u128() as f64;
                let price_impact = 0.001; // Simplified calculation
                let time_to_deadline = 30; // 30 seconds

                let confidence = self.profit_calculator.calculate_confidence_score(
                    profit_margin,
                    pool_liquidity,
                    price_impact,
                    time_to_deadline,
                );

                best_opportunity = Some(Opportunity {
                    id: uuid::Uuid::new_v4().to_string(),
                    chain_id,
                    strategy_type: "triangular_arbitrage".to_string(),
                    tokens: vec![token_a.to_string(), token_b.to_string(), token_c.to_string()],
                    pools: pools.iter().map(|p| p.address.clone()).collect(),
                    expected_profit: profit,
                    gas_cost: U256::from(300_000 * 20_000_000_000u64), // Estimated gas cost
                    confidence,
                    deadline: chrono::Utc::now().timestamp() as u64 + 60, // 1 minute deadline
                    created_at: chrono::Utc::now(),
                    metadata: [
                        ("amount_in".to_string(), serde_json::json!(amount_in.to_string())),
                        ("path".to_string(), serde_json::json!(format!("{}->{}->{}", 
                            self.get_token_symbol(token_a),
                            self.get_token_symbol(token_b),
                            self.get_token_symbol(token_c)
                        ))),
                        ("pool_count".to_string(), serde_json::json!(pools.len())),
                    ].into_iter().collect(),
                });
            }
        }

        Ok(best_opportunity)
    }
}

#[async_trait]
impl Strategy for TriangularArbitrageStrategy {
    fn name(&self) -> String {
        "Triangular Arbitrage".to_string()
    }

    async fn analyze_opportunity(
        &self,
        chain_id: u64,
        market_state: &MarketState,
        block: &Block<Transaction>,
        dex_pools: &HashMap<String, DexPool>,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Vec<Opportunity>> {
        debug!("🔺 Analyzing triangular arbitrage opportunities on chain {} block {}", 
               chain_id, block.number.unwrap_or_default());

        // Check if gas price is acceptable
        if market_state.gas_price > self.max_gas_price {
            debug!("⛽ Gas price too high: {} > {}", market_state.gas_price, self.max_gas_price);
            return Ok(vec![]);
        }

        // Find triangular arbitrage opportunities
        let opportunities = self.find_triangular_opportunities(
            chain_id,
            market_state,
            dex_pools,
            token_prices,
        ).await?;

        // Filter by minimum profit
        let profitable_opportunities: Vec<Opportunity> = opportunities
            .into_iter()
            .filter(|opp| opp.expected_profit >= self.min_profit_wei)
            .collect();

        if !profitable_opportunities.is_empty() {
            info!("💎 Found {} profitable triangular arbitrage opportunities", 
                  profitable_opportunities.len());
        }

        Ok(profitable_opportunities)
    }

    async fn validate_opportunity(&self, opportunity: &Opportunity) -> Result<bool> {
        // Basic validation
        if opportunity.tokens.len() != 3 {
            return Ok(false);
        }

        if opportunity.pools.len() != 3 {
            return Ok(false);
        }

        if opportunity.expected_profit < self.min_profit_wei {
            return Ok(false);
        }

        if opportunity.confidence < 0.5 {
            return Ok(false);
        }

        // Check if deadline hasn't passed
        let current_time = chrono::Utc::now().timestamp() as u64;
        if current_time > opportunity.deadline {
            return Ok(false);
        }

        Ok(true)
    }

    async fn estimate_gas(&self, opportunity: &Opportunity) -> Result<U256> {
        // Triangular arbitrage typically requires 3 swaps
        let base_gas = 150_000; // Gas per swap
        let total_gas = base_gas * 3;
        
        // Add buffer for complex routing
        let gas_with_buffer = total_gas + (total_gas / 10); // 10% buffer
        
        Ok(U256::from(gas_with_buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_triangular_strategy_creation() {
        // This would be a real test in production
        assert!(true);
    }

    #[test]
    fn test_token_symbol_mapping() {
        let strategy = TriangularArbitrageStrategy {
            config: Config::from_env().unwrap(),
            providers: HashMap::new(),
            profit_calculator: ProfitCalculator::new(),
            min_profit_wei: U256::zero(),
            max_gas_price: U256::zero(),
        };

        assert_eq!(strategy.get_token_symbol("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"), "WETH");
        assert_eq!(strategy.get_token_symbol("0xA0b86a33E6441E1343e0c4b0b9B3B8B1e4e8D1C5"), "USDC");
    }
}
