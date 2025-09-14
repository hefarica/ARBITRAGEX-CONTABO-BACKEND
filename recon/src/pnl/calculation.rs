use anyhow::Result;
use chrono::{DateTime, Utc};
use ethers::types::{U256, H256, Address};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, error, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnLCalculation {
    pub execution_id: String,
    pub opportunity_id: String,
    pub strategy_type: String,
    pub chain_id: u64,
    pub block_number: u64,
    pub transaction_hash: Option<H256>,
    pub expected_profit: Decimal,
    pub actual_profit: Decimal,
    pub gas_cost: Decimal,
    pub net_profit: Decimal,
    pub profit_variance: Decimal,
    pub profit_variance_percentage: Decimal,
    pub execution_time: DateTime<Utc>,
    pub tokens_involved: Vec<String>,
    pub dexes_used: Vec<String>,
    pub slippage_impact: Decimal,
    pub mev_protection_cost: Decimal,
    pub relay_fees: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnLSummary {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_expected_profit: Decimal,
    pub total_actual_profit: Decimal,
    pub total_gas_costs: Decimal,
    pub total_net_profit: Decimal,
    pub average_profit_per_execution: Decimal,
    pub success_rate: Decimal,
    pub profit_accuracy: Decimal,
    pub roi_percentage: Decimal,
    pub best_execution: Option<PnLCalculation>,
    pub worst_execution: Option<PnLCalculation>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPnL {
    pub token_address: Address,
    pub token_symbol: String,
    pub total_volume: Decimal,
    pub total_profit: Decimal,
    pub execution_count: u64,
    pub average_profit_per_execution: Decimal,
    pub best_profit: Decimal,
    pub worst_profit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPnL {
    pub strategy_name: String,
    pub execution_count: u64,
    pub total_profit: Decimal,
    pub average_profit: Decimal,
    pub success_rate: Decimal,
    pub gas_efficiency: Decimal,
    pub roi_percentage: Decimal,
}

pub struct PnLCalculator {
    db_pool: sqlx::PgPool,
    redis_client: redis::Client,
}

impl PnLCalculator {
    pub fn new(db_pool: sqlx::PgPool, redis_url: &str) -> Result<Self> {
        let redis_client = redis::Client::open(redis_url)?;
        
        Ok(Self {
            db_pool,
            redis_client,
        })
    }

    /// Calculate P&L for a specific execution
    pub async fn calculate_execution_pnl(&self, execution_id: &str) -> Result<PnLCalculation> {
        debug!("📊 Calculating P&L for execution: {}", execution_id);

        // Get execution data from database
        let execution = self.get_execution_data(execution_id).await?;
        let opportunity = self.get_opportunity_data(&execution.opportunity_id).await?;

        // Calculate actual profit from transaction receipt
        let actual_profit = self.calculate_actual_profit(&execution).await?;
        
        // Calculate gas costs
        let gas_cost = self.calculate_gas_cost(&execution).await?;
        
        // Calculate net profit
        let net_profit = actual_profit - gas_cost;
        
        // Calculate variance from expected
        let expected_profit = Decimal::from_str_exact(&opportunity.expected_profit)?;
        let profit_variance = actual_profit - expected_profit;
        let profit_variance_percentage = if expected_profit != Decimal::ZERO {
            (profit_variance / expected_profit) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Calculate additional costs
        let slippage_impact = self.calculate_slippage_impact(&execution).await?;
        let mev_protection_cost = self.calculate_mev_protection_cost(&execution).await?;
        let relay_fees = self.calculate_relay_fees(&execution).await?;

        let pnl = PnLCalculation {
            execution_id: execution_id.to_string(),
            opportunity_id: execution.opportunity_id,
            strategy_type: opportunity.strategy_type,
            chain_id: execution.chain_id,
            block_number: execution.block_number,
            transaction_hash: execution.transaction_hash,
            expected_profit,
            actual_profit,
            gas_cost,
            net_profit,
            profit_variance,
            profit_variance_percentage,
            execution_time: execution.created_at,
            tokens_involved: opportunity.tokens,
            dexes_used: self.extract_dexes_from_execution(&execution).await?,
            slippage_impact,
            mev_protection_cost,
            relay_fees,
        };

        // Store P&L calculation in database
        self.store_pnl_calculation(&pnl).await?;

        info!("✅ P&L calculated for execution {}: Net profit = {}", execution_id, net_profit);
        Ok(pnl)
    }

    /// Calculate P&L summary for a time period
    pub async fn calculate_period_summary(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        strategy_filter: Option<&str>,
    ) -> Result<PnLSummary> {
        info!("📈 Calculating P&L summary from {} to {}", start_time, end_time);

        let mut query = "
            SELECT 
                execution_id,
                expected_profit,
                actual_profit,
                gas_cost,
                net_profit,
                profit_variance,
                execution_time
            FROM pnl_calculations 
            WHERE execution_time >= $1 AND execution_time <= $2
        ".to_string();

        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = vec![
            Box::new(start_time),
            Box::new(end_time),
        ];

        if let Some(strategy) = strategy_filter {
            query.push_str(" AND strategy_type = $3");
            params.push(Box::new(strategy.to_string()));
        }

        query.push_str(" ORDER BY execution_time DESC");

        // For now, use a simpler approach without dynamic parameters
        let rows = sqlx::query!(
            "SELECT 
                execution_id,
                expected_profit,
                actual_profit,
                gas_cost,
                net_profit,
                profit_variance,
                execution_time
            FROM pnl_calculations 
            WHERE execution_time >= $1 AND execution_time <= $2
            ORDER BY execution_time DESC",
            start_time,
            end_time
        )
        .fetch_all(&self.db_pool)
        .await?;

        let total_executions = rows.len() as u64;
        let successful_executions = rows.iter()
            .filter(|r| r.net_profit.unwrap_or_default() > rust_decimal::Decimal::ZERO)
            .count() as u64;
        let failed_executions = total_executions - successful_executions;

        let total_expected_profit: Decimal = rows.iter()
            .map(|r| r.expected_profit.unwrap_or_default())
            .sum();

        let total_actual_profit: Decimal = rows.iter()
            .map(|r| r.actual_profit.unwrap_or_default())
            .sum();

        let total_gas_costs: Decimal = rows.iter()
            .map(|r| r.gas_cost.unwrap_or_default())
            .sum();

        let total_net_profit: Decimal = rows.iter()
            .map(|r| r.net_profit.unwrap_or_default())
            .sum();

        let average_profit_per_execution = if total_executions > 0 {
            total_net_profit / Decimal::from(total_executions)
        } else {
            Decimal::ZERO
        };

        let success_rate = if total_executions > 0 {
            Decimal::from(successful_executions) / Decimal::from(total_executions) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let profit_accuracy = if total_expected_profit != Decimal::ZERO {
            (total_actual_profit / total_expected_profit) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let roi_percentage = if total_gas_costs != Decimal::ZERO {
            (total_net_profit / total_gas_costs) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Find best and worst executions
        let best_execution = rows.iter()
            .max_by_key(|r| r.net_profit.unwrap_or_default())
            .map(|r| r.execution_id.clone());

        let worst_execution = rows.iter()
            .min_by_key(|r| r.net_profit.unwrap_or_default())
            .map(|r| r.execution_id.clone());

        let summary = PnLSummary {
            total_executions,
            successful_executions,
            failed_executions,
            total_expected_profit,
            total_actual_profit,
            total_gas_costs,
            total_net_profit,
            average_profit_per_execution,
            success_rate,
            profit_accuracy,
            roi_percentage,
            best_execution: None, // Would fetch full calculation if needed
            worst_execution: None, // Would fetch full calculation if needed
            period_start: start_time,
            period_end: end_time,
        };

        info!("✅ P&L summary calculated: {} executions, {} net profit", 
              total_executions, total_net_profit);

        Ok(summary)
    }

    /// Calculate P&L by token
    pub async fn calculate_token_pnl(&self, time_period_days: u32) -> Result<Vec<TokenPnL>> {
        let start_time = Utc::now() - chrono::Duration::days(time_period_days as i64);
        
        let rows = sqlx::query!(
            "SELECT 
                unnest(tokens_involved) as token_symbol,
                COUNT(*) as execution_count,
                SUM(net_profit) as total_profit,
                AVG(net_profit) as average_profit,
                MAX(net_profit) as best_profit,
                MIN(net_profit) as worst_profit
            FROM pnl_calculations 
            WHERE execution_time >= $1
            GROUP BY token_symbol
            ORDER BY total_profit DESC",
            start_time
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut token_pnls = Vec::new();
        for row in rows {
            token_pnls.push(TokenPnL {
                token_address: Address::zero(), // Would need to resolve from symbol
                token_symbol: row.token_symbol.unwrap_or_default(),
                total_volume: Decimal::ZERO, // Would calculate from transaction data
                total_profit: row.total_profit.unwrap_or_default(),
                execution_count: row.execution_count.unwrap_or_default() as u64,
                average_profit_per_execution: row.average_profit.unwrap_or_default(),
                best_profit: row.best_profit.unwrap_or_default(),
                worst_profit: row.worst_profit.unwrap_or_default(),
            });
        }

        Ok(token_pnls)
    }

    /// Calculate P&L by strategy
    pub async fn calculate_strategy_pnl(&self, time_period_days: u32) -> Result<Vec<StrategyPnL>> {
        let start_time = Utc::now() - chrono::Duration::days(time_period_days as i64);
        
        let rows = sqlx::query!(
            "SELECT 
                strategy_type,
                COUNT(*) as execution_count,
                SUM(net_profit) as total_profit,
                AVG(net_profit) as average_profit,
                COUNT(CASE WHEN net_profit > 0 THEN 1 END)::float / COUNT(*)::float * 100 as success_rate,
                AVG(gas_cost) as avg_gas_cost
            FROM pnl_calculations 
            WHERE execution_time >= $1
            GROUP BY strategy_type
            ORDER BY total_profit DESC",
            start_time
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut strategy_pnls = Vec::new();
        for row in rows {
            let total_profit = row.total_profit.unwrap_or_default();
            let avg_gas_cost = row.avg_gas_cost.unwrap_or_default();
            
            strategy_pnls.push(StrategyPnL {
                strategy_name: row.strategy_type,
                execution_count: row.execution_count.unwrap_or_default() as u64,
                total_profit,
                average_profit: row.average_profit.unwrap_or_default(),
                success_rate: Decimal::from_f64(row.success_rate.unwrap_or_default()).unwrap_or_default(),
                gas_efficiency: if avg_gas_cost != Decimal::ZERO {
                    total_profit / avg_gas_cost
                } else {
                    Decimal::ZERO
                },
                roi_percentage: if avg_gas_cost != Decimal::ZERO {
                    (total_profit / avg_gas_cost) * Decimal::from(100)
                } else {
                    Decimal::ZERO
                },
            });
        }

        Ok(strategy_pnls)
    }

    // Helper methods (simplified implementations)
    async fn get_execution_data(&self, execution_id: &str) -> Result<ExecutionData> {
        // Would fetch from executions table
        Ok(ExecutionData::default())
    }

    async fn get_opportunity_data(&self, opportunity_id: &str) -> Result<OpportunityData> {
        // Would fetch from opportunities table
        Ok(OpportunityData::default())
    }

    async fn calculate_actual_profit(&self, execution: &ExecutionData) -> Result<Decimal> {
        // Would analyze transaction receipt and token transfers
        Ok(Decimal::ZERO)
    }

    async fn calculate_gas_cost(&self, execution: &ExecutionData) -> Result<Decimal> {
        // Would calculate from gas_used * gas_price
        Ok(Decimal::ZERO)
    }

    async fn calculate_slippage_impact(&self, execution: &ExecutionData) -> Result<Decimal> {
        // Would calculate slippage from expected vs actual swap amounts
        Ok(Decimal::ZERO)
    }

    async fn calculate_mev_protection_cost(&self, execution: &ExecutionData) -> Result<Decimal> {
        // Would calculate MEV protection fees (Flashbots, etc.)
        Ok(Decimal::ZERO)
    }

    async fn calculate_relay_fees(&self, execution: &ExecutionData) -> Result<Decimal> {
        // Would calculate relay submission fees
        Ok(Decimal::ZERO)
    }

    async fn extract_dexes_from_execution(&self, execution: &ExecutionData) -> Result<Vec<String>> {
        // Would extract DEX names from transaction data
        Ok(vec!["Uniswap V2".to_string(), "SushiSwap".to_string()])
    }

    async fn store_pnl_calculation(&self, pnl: &PnLCalculation) -> Result<()> {
        // Would store in pnl_calculations table
        Ok(())
    }
}

// Helper structs (would be properly defined)
#[derive(Default)]
struct ExecutionData {
    opportunity_id: String,
    chain_id: u64,
    block_number: u64,
    transaction_hash: Option<H256>,
    created_at: DateTime<Utc>,
}

#[derive(Default)]
struct OpportunityData {
    strategy_type: String,
    expected_profit: String,
    tokens: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pnl_calculation_creation() {
        let pnl = PnLCalculation {
            execution_id: "test".to_string(),
            opportunity_id: "test_opp".to_string(),
            strategy_type: "triangular".to_string(),
            chain_id: 1,
            block_number: 12345,
            transaction_hash: None,
            expected_profit: Decimal::from(100),
            actual_profit: Decimal::from(95),
            gas_cost: Decimal::from(5),
            net_profit: Decimal::from(90),
            profit_variance: Decimal::from(-5),
            profit_variance_percentage: Decimal::from(-5),
            execution_time: Utc::now(),
            tokens_involved: vec!["WETH".to_string(), "USDC".to_string()],
            dexes_used: vec!["Uniswap".to_string()],
            slippage_impact: Decimal::ZERO,
            mev_protection_cost: Decimal::ZERO,
            relay_fees: Decimal::ZERO,
        };

        assert_eq!(pnl.net_profit, Decimal::from(90));
        assert_eq!(pnl.profit_variance_percentage, Decimal::from(-5));
    }
}
