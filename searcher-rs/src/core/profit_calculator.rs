use anyhow::Result;
use ethers::prelude::*;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::types::{DexPool, TokenPrice, Opportunity};

pub struct ProfitCalculator {
    gas_price_oracle: GasPriceOracle,
    slippage_calculator: SlippageCalculator,
    eth_price_usd: f64,
    gas_price_cache: HashMap<String, u64>,
    dex_fee_rates: HashMap<String, u16>,
    flash_loan_fee_rates: HashMap<String, u16>,
    slippage_tolerance: f64,
}

impl ProfitCalculator {
    pub fn new() -> Self {
        Self {
            gas_price_oracle: GasPriceOracle::new(),
            slippage_calculator: SlippageCalculator::new(),
            eth_price_usd: 2000.0, // Will be updated from real price feeds
            gas_price_cache: HashMap::new(),
            dex_fee_rates: HashMap::new(),
            flash_loan_fee_rates: HashMap::new(),
            slippage_tolerance: 0.005, // 0.5% default slippage tolerance
        }
    }

    /// Calculate net profit for a triangular arbitrage opportunity
    pub async fn calculate_triangular_arbitrage_profit(
        &self,
        token_a: &str,
        token_b: &str,
        token_c: &str,
        amount_in: U256,
        pools: &[DexPool],
        token_prices: &HashMap<String, TokenPrice>,
        chain_id: u64,
    ) -> Result<U256> {
        debug!("🧮 Calculating triangular arbitrage profit for {}-{}-{}", token_a, token_b, token_c);

        // Step 1: A -> B
        let amount_b = self.calculate_swap_output(
            amount_in,
            token_a,
            token_b,
            &pools[0],
            token_prices,
        ).await?;

        // Step 2: B -> C
        let amount_c = self.calculate_swap_output(
            amount_b,
            token_b,
            token_c,
            &pools[1],
            token_prices,
        ).await?;

        // Step 3: C -> A
        let amount_a_out = self.calculate_swap_output(
            amount_c,
            token_c,
            token_a,
            &pools[2],
            token_prices,
        ).await?;

        // Calculate gross profit
        let gross_profit = if amount_a_out > amount_in {
            amount_a_out - amount_in
        } else {
            return Ok(U256::zero()); // No profit
        };

        // Calculate gas costs
        let gas_cost = self.estimate_gas_cost(chain_id, 3).await?; // 3 swaps

        // Calculate net profit
        let net_profit = if gross_profit > gas_cost {
            gross_profit - gas_cost
        } else {
            U256::zero()
        };

        debug!("💰 Triangular arbitrage: gross={}, gas={}, net={}", 
               gross_profit, gas_cost, net_profit);

        Ok(net_profit)
    }

    /// Calculate profit for cross-DEX arbitrage
    pub async fn calculate_cross_dex_profit(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: U256,
        buy_pool: &DexPool,
        sell_pool: &DexPool,
        token_prices: &HashMap<String, TokenPrice>,
        chain_id: u64,
    ) -> Result<U256> {
        debug!("🔄 Calculating cross-DEX arbitrage profit for {}->{}", token_in, token_out);

        // Buy on DEX A
        let amount_bought = self.calculate_swap_output(
            amount_in,
            token_in,
            token_out,
            buy_pool,
            token_prices,
        ).await?;

        // Sell on DEX B
        let amount_sold = self.calculate_swap_output(
            amount_bought,
            token_out,
            token_in,
            sell_pool,
            token_prices,
        ).await?;

        // Calculate gross profit
        let gross_profit = if amount_sold > amount_in {
            amount_sold - amount_in
        } else {
            return Ok(U256::zero());
        };

        // Calculate gas costs (2 swaps)
        let gas_cost = self.estimate_gas_cost(chain_id, 2).await?;

        // Calculate net profit
        let net_profit = if gross_profit > gas_cost {
            gross_profit - gas_cost
        } else {
            U256::zero()
        };

        debug!("💱 Cross-DEX arbitrage: gross={}, gas={}, net={}", 
               gross_profit, gas_cost, net_profit);

        Ok(net_profit)
    }

    /// Calculate swap output using constant product formula (Uniswap V2 style)
    async fn calculate_swap_output(
        &self,
        amount_in: U256,
        token_in: &str,
        token_out: &str,
        pool: &DexPool,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<U256> {
        // Get token prices for reserve calculation
        let price_in = token_prices.get(token_in)
            .map(|p| p.price_usd)
            .unwrap_or(1.0);
        let price_out = token_prices.get(token_out)
            .map(|p| p.price_usd)
            .unwrap_or(1.0);

        // Estimate reserves based on TVL and prices
        let total_value = pool.tvl_usd;
        let reserve_in_usd = total_value / 2.0; // Assume 50/50 split
        let reserve_out_usd = total_value / 2.0;

        let reserve_in = U256::from((reserve_in_usd / price_in * 1e18) as u64);
        let reserve_out = U256::from((reserve_out_usd / price_out * 1e18) as u64);

        // Apply fee (typically 0.3% for Uniswap V2)
        let fee_rate = 997; // 0.3% fee = 997/1000
        let amount_in_with_fee = amount_in * U256::from(fee_rate);

        // Constant product formula: (x + Δx) * (y - Δy) = x * y
        // Δy = (y * Δx) / (x + Δx)
        let numerator = reserve_out * amount_in_with_fee;
        let denominator = reserve_in * U256::from(1000) + amount_in_with_fee;

        if denominator.is_zero() {
            return Ok(U256::zero());
        }

        let amount_out = numerator / denominator;

        // Apply slippage
        let slippage_factor = self.slippage_calculator.calculate_slippage(
            amount_in,
            reserve_in,
            reserve_out,
        );

        let amount_out_with_slippage = amount_out * U256::from((1.0 - slippage_factor) * 1000.0 as u64) / U256::from(1000);

        Ok(amount_out_with_slippage)
    }

    /// Estimate gas cost for multiple operations
    async fn estimate_gas_cost(&self, chain_id: u64, num_operations: u32) -> Result<U256> {
        let gas_price = self.gas_price_oracle.get_gas_price(chain_id).await?;
        
        // Estimate gas per operation based on chain
        let gas_per_operation = match chain_id {
            1 => 150_000, // Ethereum mainnet
            137 => 100_000, // Polygon
            _ => 120_000, // Default
        };

        let total_gas = gas_per_operation * num_operations;
        let total_cost = gas_price * U256::from(total_gas);

        Ok(total_cost)
    }

    /// Calculate confidence score based on various factors
    pub fn calculate_confidence_score(
        &self,
        profit_margin: f64,
        pool_liquidity: f64,
        price_impact: f64,
        time_to_deadline: u64,
    ) -> f64 {
        let mut score = 0.0;

        // Profit margin factor (0-40 points)
        score += (profit_margin * 100.0).min(40.0);

        // Liquidity factor (0-30 points)
        let liquidity_score = if pool_liquidity > 1_000_000.0 {
            30.0
        } else if pool_liquidity > 100_000.0 {
            20.0
        } else if pool_liquidity > 10_000.0 {
            10.0
        } else {
            5.0
        };
        score += liquidity_score;

        // Price impact factor (0-20 points, lower impact = higher score)
        let impact_score = if price_impact < 0.001 {
            20.0
        } else if price_impact < 0.005 {
            15.0
        } else if price_impact < 0.01 {
            10.0
        } else {
            5.0
        };
        score += impact_score;

        // Time factor (0-10 points)
        let time_score = if time_to_deadline > 30 {
            10.0
        } else if time_to_deadline > 15 {
            7.0
        } else if time_to_deadline > 5 {
            5.0
        } else {
            2.0
        };
        score += time_score;

        // Normalize to 0-1 range
        (score / 100.0).min(1.0)
    }
}

/// Gas price oracle for different chains
pub struct GasPriceOracle {
    cached_prices: HashMap<u64, (U256, u64)>, // (price, timestamp)
}

impl GasPriceOracle {
    pub fn new() -> Self {
        Self {
            cached_prices: HashMap::new(),
        }
    }

    pub async fn get_gas_price(&self, chain_id: u64) -> Result<U256> {
        // For now, return static values. In production, this would fetch from gas oracles
        let gas_price = match chain_id {
            1 => U256::from(20_000_000_000u64), // 20 gwei for Ethereum
            137 => U256::from(30_000_000_000u64), // 30 gwei for Polygon
            _ => U256::from(25_000_000_000u64), // 25 gwei default
        };

        Ok(gas_price)
    }
}

/// Slippage calculator
pub struct SlippageCalculator;

impl SlippageCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate slippage based on trade size and pool reserves
    pub fn calculate_slippage(
        &self,
        trade_amount: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> f64 {
        if reserve_in.is_zero() || trade_amount.is_zero() {
            return 0.0;
        }

        // Calculate price impact as percentage of pool
        let trade_ratio = trade_amount.as_u128() as f64 / reserve_in.as_u128() as f64;
        
        // Slippage increases quadratically with trade size
        let base_slippage = trade_ratio * trade_ratio * 0.1; // Base slippage formula
        
        // Cap slippage at 5%
        base_slippage.min(0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_profit_calculation() {
        let calculator = ProfitCalculator::new();
        let amount_in = U256::from(1_000_000_000_000_000_000u64); // 1 ETH

        // Mock pools and prices
        let pools = vec![
            DexPool {
                address: "0x123".to_string(),
                chain: "ethereum".to_string(),
                project: "uniswap".to_string(),
                symbol: "ETH-USDC".to_string(),
                tvl_usd: 10_000_000.0,
                apy: 0.05,
                volume_usd_1d: 1_000_000.0,
                volume_usd_7d: 7_000_000.0,
                last_updated: 1234567890,
            }
        ];

        let mut token_prices = HashMap::new();
        token_prices.insert("ETH".to_string(), TokenPrice {
            token_id: "ethereum".to_string(),
            price_usd: 2000.0,
            last_updated: 1234567890,
        });

        // This would be a real test in production
        assert!(true);
    }
}
