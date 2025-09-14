use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use async_trait::async_trait;

use crate::strategies::Strategy;
use crate::types::{Config, Opportunity, MarketState, DexPool, TokenPrice};
use crate::core::profit_calculator::ProfitCalculator;

pub struct FlashLoanArbitrageStrategy {
    config: Config,
    providers: HashMap<u64, Arc<Provider<Http>>>,
    profit_calculator: ProfitCalculator,
    min_profit_wei: U256,
    max_gas_price: U256,
    flash_loan_providers: HashMap<u64, Vec<FlashLoanProvider>>,
}

#[derive(Debug, Clone)]
pub struct FlashLoanProvider {
    pub name: String,
    pub address: String,
    pub fee_rate: f64, // Fee as percentage (e.g., 0.0009 for 0.09%)
    pub max_loan_amount: U256,
    pub supported_tokens: Vec<String>,
}

impl FlashLoanArbitrageStrategy {
    pub async fn new(
        config: Config,
        providers: HashMap<u64, Arc<Provider<Http>>>,
    ) -> Result<Self> {
        let strategy_config = config.strategies
            .iter()
            .find(|s| s.name == "flash_loan_arbitrage")
            .ok_or_else(|| anyhow::anyhow!("Flash loan arbitrage strategy not found in config"))?;

        let mut flash_loan_providers = HashMap::new();
        
        // Initialize flash loan providers for each chain
        flash_loan_providers.insert(1, vec![ // Ethereum
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(),
                fee_rate: 0.0009, // 0.09%
                max_loan_amount: U256::from_dec_str("1000000000000000000000000").unwrap(), // 1M tokens
                supported_tokens: vec![
                    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                    "0xA0b86a33E6441E1343e0c4b0b9B3B8B1e4e8D1C5".to_string(), // USDC
                    "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(), // USDT
                    "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(), // DAI
                ],
            },
            FlashLoanProvider {
                name: "Balancer V2".to_string(),
                address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
                fee_rate: 0.0, // No fee for flash loans
                max_loan_amount: U256::from_dec_str("10000000000000000000000000").unwrap(), // 10M tokens
                supported_tokens: vec![
                    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
                    "0xA0b86a33E6441E1343e0c4b0b9B3B8B1e4e8D1C5".to_string(), // USDC
                    "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(), // WBTC
                ],
            },
        ]);

        flash_loan_providers.insert(137, vec![ // Polygon
            FlashLoanProvider {
                name: "Aave V3 Polygon".to_string(),
                address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string(),
                fee_rate: 0.0009, // 0.09%
                max_loan_amount: U256::from_dec_str("1000000000000000000000000").unwrap(),
                supported_tokens: vec![
                    "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".to_string(), // WMATIC
                    "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(), // USDC
                    "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".to_string(), // WETH
                ],
            },
        ]);

        Ok(Self {
            config: config.clone(),
            providers,
            profit_calculator: ProfitCalculator::new(),
            min_profit_wei: U256::from(strategy_config.min_profit_wei),
            max_gas_price: strategy_config.max_gas_price,
            flash_loan_providers,
        })
    }

    /// Find flash loan arbitrage opportunities
    async fn find_flash_loan_opportunities(
        &self,
        chain_id: u64,
        market_state: &MarketState,
        dex_pools: &HashMap<String, DexPool>,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Vec<Opportunity>> {
        let mut opportunities = Vec::new();

        let flash_providers = match self.flash_loan_providers.get(&chain_id) {
            Some(providers) => providers,
            None => {
                debug!("No flash loan providers configured for chain {}", chain_id);
                return Ok(opportunities);
            }
        };

        debug!("🔍 Scanning flash loan opportunities on chain {} with {} providers", 
               chain_id, flash_providers.len());

        // Get all available pools for the chain
        let chain_pools: Vec<&DexPool> = dex_pools.values()
            .filter(|pool| self.get_chain_name(chain_id) == pool.chain)
            .collect();

        // For each flash loan provider
        for provider in flash_providers {
            // For each supported token
            for token in &provider.supported_tokens {
                // Find arbitrage opportunities using this token as flash loan
                if let Some(opps) = self.find_token_arbitrage_opportunities(
                    chain_id,
                    token,
                    provider,
                    &chain_pools,
                    market_state,
                    token_prices,
                ).await? {
                    opportunities.extend(opps);
                }
            }
        }

        info!("💡 Found {} flash loan arbitrage opportunities on chain {}", 
              opportunities.len(), chain_id);

        Ok(opportunities)
    }

    /// Find arbitrage opportunities for a specific token using flash loans
    async fn find_token_arbitrage_opportunities(
        &self,
        chain_id: u64,
        flash_token: &str,
        provider: &FlashLoanProvider,
        pools: &[&DexPool],
        market_state: &MarketState,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Option<Vec<Opportunity>>> {
        let mut opportunities = Vec::new();
        let token_symbol = self.get_token_symbol(flash_token);

        // Find pools that contain our flash loan token
        let relevant_pools: Vec<&DexPool> = pools.iter()
            .filter(|pool| {
                let symbol_lower = pool.symbol.to_lowercase();
                symbol_lower.contains(&token_symbol.to_lowercase())
            })
            .cloned()
            .collect();

        if relevant_pools.len() < 2 {
            return Ok(None); // Need at least 2 pools for arbitrage
        }

        debug!("🔄 Checking {} pools for {} flash loan arbitrage", 
               relevant_pools.len(), token_symbol);

        // Compare prices across pools to find arbitrage opportunities
        for i in 0..relevant_pools.len() {
            for j in (i + 1)..relevant_pools.len() {
                let pool_buy = relevant_pools[i];
                let pool_sell = relevant_pools[j];

                // Check both directions: buy from i, sell to j AND buy from j, sell to i
                if let Some(opp) = self.check_flash_loan_arbitrage(
                    chain_id,
                    flash_token,
                    pool_buy,
                    pool_sell,
                    provider,
                    market_state,
                    token_prices,
                ).await? {
                    opportunities.push(opp);
                }

                if let Some(opp) = self.check_flash_loan_arbitrage(
                    chain_id,
                    flash_token,
                    pool_sell,
                    pool_buy,
                    provider,
                    market_state,
                    token_prices,
                ).await? {
                    opportunities.push(opp);
                }
            }
        }

        Ok(if opportunities.is_empty() { None } else { Some(opportunities) })
    }

    /// Check a specific flash loan arbitrage opportunity
    async fn check_flash_loan_arbitrage(
        &self,
        chain_id: u64,
        flash_token: &str,
        buy_pool: &DexPool,
        sell_pool: &DexPool,
        provider: &FlashLoanProvider,
        market_state: &MarketState,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Option<Opportunity>> {
        // Test different flash loan amounts
        let test_amounts = vec![
            U256::from(1_000_000_000_000_000_000u64),   // 1 token
            U256::from(10_000_000_000_000_000_000u64),  // 10 tokens
            U256::from(100_000_000_000_000_000_000u64), // 100 tokens
            U256::from(1_000_000_000_000_000_000_000u64), // 1000 tokens
        ];

        let mut best_opportunity: Option<Opportunity> = None;
        let mut best_profit = U256::zero();

        for flash_amount in test_amounts {
            // Don't exceed provider's max loan amount
            if flash_amount > provider.max_loan_amount {
                continue;
            }

            // Calculate potential profit
            let profit = self.calculate_flash_loan_profit(
                flash_token,
                flash_amount,
                buy_pool,
                sell_pool,
                provider,
                token_prices,
                chain_id,
            ).await?;

            if profit > best_profit && profit >= self.min_profit_wei {
                best_profit = profit;

                // Calculate confidence score
                let avg_liquidity = (buy_pool.tvl_usd + sell_pool.tvl_usd) / 2.0;
                let profit_margin = profit.as_u128() as f64 / flash_amount.as_u128() as f64;
                let price_impact = self.estimate_price_impact(flash_amount, avg_liquidity);
                let time_to_deadline = 45; // 45 seconds for flash loan execution

                let confidence = self.profit_calculator.calculate_confidence_score(
                    profit_margin,
                    avg_liquidity,
                    price_impact,
                    time_to_deadline,
                );

                best_opportunity = Some(Opportunity {
                    id: uuid::Uuid::new_v4().to_string(),
                    chain_id,
                    strategy_type: "flash_loan_arbitrage".to_string(),
                    tokens: vec![flash_token.to_string()],
                    pools: vec![buy_pool.address.clone(), sell_pool.address.clone()],
                    expected_profit: profit,
                    gas_cost: U256::from(500_000 * 30_000_000_000u64), // Higher gas for flash loans
                    confidence,
                    deadline: chrono::Utc::now().timestamp() as u64 + 90, // 90 seconds
                    created_at: chrono::Utc::now(),
                    metadata: [
                        ("flash_amount".to_string(), serde_json::json!(flash_amount.to_string())),
                        ("flash_provider".to_string(), serde_json::json!(provider.name)),
                        ("buy_pool".to_string(), serde_json::json!(buy_pool.project)),
                        ("sell_pool".to_string(), serde_json::json!(sell_pool.project)),
                        ("flash_fee".to_string(), serde_json::json!(provider.fee_rate)),
                    ].into_iter().collect(),
                });
            }
        }

        Ok(best_opportunity)
    }

    /// Calculate profit for a flash loan arbitrage
    async fn calculate_flash_loan_profit(
        &self,
        flash_token: &str,
        flash_amount: U256,
        buy_pool: &DexPool,
        sell_pool: &DexPool,
        provider: &FlashLoanProvider,
        token_prices: &HashMap<String, TokenPrice>,
        chain_id: u64,
    ) -> Result<U256> {
        // Step 1: Calculate how much we can buy with flash loan
        let amount_bought = self.simulate_swap(
            flash_amount,
            flash_token,
            buy_pool,
            token_prices,
            true, // buying
        ).await?;

        // Step 2: Calculate how much we get from selling
        let amount_sold = self.simulate_swap(
            amount_bought,
            flash_token,
            sell_pool,
            token_prices,
            false, // selling
        ).await?;

        // Step 3: Calculate flash loan fee
        let flash_fee = U256::from((flash_amount.as_u128() as f64 * provider.fee_rate) as u128);

        // Step 4: Calculate gas costs
        let gas_cost = self.estimate_flash_loan_gas_cost(chain_id).await?;

        // Step 5: Calculate net profit
        let total_costs = flash_amount + flash_fee + gas_cost;
        
        let net_profit = if amount_sold > total_costs {
            amount_sold - total_costs
        } else {
            U256::zero()
        };

        debug!("💰 Flash loan profit calculation: bought={}, sold={}, fee={}, gas={}, net={}", 
               amount_bought, amount_sold, flash_fee, gas_cost, net_profit);

        Ok(net_profit)
    }

    /// Simulate a swap to estimate output
    async fn simulate_swap(
        &self,
        amount_in: U256,
        token: &str,
        pool: &DexPool,
        token_prices: &HashMap<String, TokenPrice>,
        is_buying: bool,
    ) -> Result<U256> {
        // Get token price
        let token_symbol = self.get_token_symbol(token);
        let token_price = token_prices.get(&token_symbol.to_lowercase())
            .map(|p| p.price_usd)
            .unwrap_or(1.0);

        // Estimate pool reserves based on TVL
        let total_value = pool.tvl_usd;
        let reserve_token = U256::from((total_value / 2.0 / token_price * 1e18) as u64);
        let reserve_other = U256::from((total_value / 2.0 * 1e18) as u64);

        // Apply constant product formula with fees
        let fee_rate = 997; // 0.3% fee
        let amount_in_with_fee = amount_in * U256::from(fee_rate);

        let (reserve_in, reserve_out) = if is_buying {
            (reserve_other, reserve_token)
        } else {
            (reserve_token, reserve_other)
        };

        let numerator = reserve_out * amount_in_with_fee;
        let denominator = reserve_in * U256::from(1000) + amount_in_with_fee;

        if denominator.is_zero() {
            return Ok(U256::zero());
        }

        let amount_out = numerator / denominator;

        // Apply slippage
        let slippage = self.calculate_slippage(amount_in, reserve_in);
        let amount_out_with_slippage = amount_out * U256::from((1.0 - slippage) * 1000.0 as u64) / U256::from(1000);

        Ok(amount_out_with_slippage)
    }

    /// Calculate slippage for a trade
    fn calculate_slippage(&self, trade_amount: U256, reserve: U256) -> f64 {
        if reserve.is_zero() || trade_amount.is_zero() {
            return 0.0;
        }

        let trade_ratio = trade_amount.as_u128() as f64 / reserve.as_u128() as f64;
        (trade_ratio * trade_ratio * 0.1).min(0.05) // Max 5% slippage
    }

    /// Estimate price impact
    fn estimate_price_impact(&self, amount: U256, liquidity_usd: f64) -> f64 {
        let amount_usd = amount.as_u128() as f64 / 1e18; // Assume 18 decimals
        (amount_usd / liquidity_usd).min(0.1) // Max 10% impact
    }

    /// Estimate gas cost for flash loan execution
    async fn estimate_flash_loan_gas_cost(&self, chain_id: u64) -> Result<U256> {
        let gas_price = match chain_id {
            1 => U256::from(30_000_000_000u64), // 30 gwei for Ethereum
            137 => U256::from(40_000_000_000u64), // 40 gwei for Polygon
            _ => U256::from(35_000_000_000u64), // 35 gwei default
        };

        let gas_limit = 800_000; // Flash loans typically use more gas
        Ok(gas_price * U256::from(gas_limit))
    }

    /// Get chain name from chain ID
    fn get_chain_name(&self, chain_id: u64) -> String {
        match chain_id {
            1 => "ethereum".to_string(),
            137 => "polygon".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Get token symbol from address
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
}

#[async_trait]
impl Strategy for FlashLoanArbitrageStrategy {
    fn name(&self) -> String {
        "Flash Loan Arbitrage".to_string()
    }

    async fn analyze_opportunity(
        &self,
        chain_id: u64,
        market_state: &MarketState,
        block: &Block<Transaction>,
        dex_pools: &HashMap<String, DexPool>,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Vec<Opportunity>> {
        debug!("⚡ Analyzing flash loan arbitrage opportunities on chain {} block {}", 
               chain_id, block.number.unwrap_or_default());

        // Check if gas price is acceptable
        if market_state.gas_price > self.max_gas_price {
            debug!("⛽ Gas price too high for flash loans: {} > {}", 
                   market_state.gas_price, self.max_gas_price);
            return Ok(vec![]);
        }

        // Find flash loan arbitrage opportunities
        let opportunities = self.find_flash_loan_opportunities(
            chain_id,
            market_state,
            dex_pools,
            token_prices,
        ).await?;

        // Filter by minimum profit and confidence
        let profitable_opportunities: Vec<Opportunity> = opportunities
            .into_iter()
            .filter(|opp| opp.expected_profit >= self.min_profit_wei && opp.confidence >= 0.6)
            .collect();

        if !profitable_opportunities.is_empty() {
            info!("⚡ Found {} profitable flash loan arbitrage opportunities", 
                  profitable_opportunities.len());
        }

        Ok(profitable_opportunities)
    }

    async fn validate_opportunity(&self, opportunity: &Opportunity) -> Result<bool> {
        // Validate flash loan specific requirements
        if opportunity.pools.len() != 2 {
            return Ok(false);
        }

        if opportunity.expected_profit < self.min_profit_wei {
            return Ok(false);
        }

        if opportunity.confidence < 0.6 {
            return Ok(false);
        }

        // Check deadline
        let current_time = chrono::Utc::now().timestamp() as u64;
        if current_time > opportunity.deadline {
            return Ok(false);
        }

        // Validate flash loan amount is within limits
        if let Some(flash_amount_str) = opportunity.metadata.get("flash_amount") {
            if let Some(amount_str) = flash_amount_str.as_str() {
                if let Ok(flash_amount) = U256::from_dec_str(amount_str) {
                    if let Some(providers) = self.flash_loan_providers.get(&opportunity.chain_id) {
                        let token = &opportunity.tokens[0];
                        for provider in providers {
                            if provider.supported_tokens.contains(token) && 
                               flash_amount <= provider.max_loan_amount {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    async fn estimate_gas(&self, opportunity: &Opportunity) -> Result<U256> {
        // Flash loans require more gas due to complex execution
        let base_gas = 600_000; // Base gas for flash loan execution
        let swap_gas = 150_000 * 2; // Two swaps
        let total_gas = base_gas + swap_gas;
        
        // Add buffer for complex routing and callbacks
        let gas_with_buffer = total_gas + (total_gas / 5); // 20% buffer
        
        Ok(U256::from(gas_with_buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_loan_provider_creation() {
        let provider = FlashLoanProvider {
            name: "Test Provider".to_string(),
            address: "0x123".to_string(),
            fee_rate: 0.001,
            max_loan_amount: U256::from(1000000),
            supported_tokens: vec!["0xETH".to_string()],
        };

        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.fee_rate, 0.001);
    }

    #[test]
    fn test_slippage_calculation() {
        let strategy = FlashLoanArbitrageStrategy {
            config: Config::from_env().unwrap(),
            providers: HashMap::new(),
            profit_calculator: ProfitCalculator::new(),
            min_profit_wei: U256::zero(),
            max_gas_price: U256::zero(),
            flash_loan_providers: HashMap::new(),
        };

        let slippage = strategy.calculate_slippage(
            U256::from(1000), 
            U256::from(100000)
        );
        
        assert!(slippage > 0.0);
        assert!(slippage < 0.05);
    }
}
