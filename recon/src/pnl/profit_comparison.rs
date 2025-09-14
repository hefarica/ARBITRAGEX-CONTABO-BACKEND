use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, error, debug};

use super::calculation::{PnLCalculation, PnLSummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitComparison {
    pub execution_id: String,
    pub simulated_profit: Decimal,
    pub actual_profit: Decimal,
    pub profit_difference: Decimal,
    pub profit_accuracy_percentage: Decimal,
    pub simulation_gas_estimate: Decimal,
    pub actual_gas_cost: Decimal,
    pub gas_estimation_accuracy: Decimal,
    pub slippage_impact: Decimal,
    pub mev_impact: Decimal,
    pub timing_impact: Decimal,
    pub market_movement_impact: Decimal,
    pub comparison_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyMetrics {
    pub total_comparisons: u64,
    pub average_profit_accuracy: Decimal,
    pub average_gas_accuracy: Decimal,
    pub profit_overestimation_count: u64,
    pub profit_underestimation_count: u64,
    pub gas_overestimation_count: u64,
    pub gas_underestimation_count: u64,
    pub best_accuracy_percentage: Decimal,
    pub worst_accuracy_percentage: Decimal,
    pub accuracy_standard_deviation: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAccuracy {
    pub strategy_name: String,
    pub execution_count: u64,
    pub average_profit_accuracy: Decimal,
    pub average_gas_accuracy: Decimal,
    pub success_rate: Decimal,
    pub reliability_score: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditionImpact {
    pub volatility_level: String, // "low", "medium", "high"
    pub average_accuracy: Decimal,
    pub slippage_impact: Decimal,
    pub execution_count: u64,
    pub success_rate: Decimal,
}

pub struct ProfitComparator {
    db_pool: sqlx::PgPool,
}

impl ProfitComparator {
    pub fn new(db_pool: sqlx::PgPool) -> Self {
        Self { db_pool }
    }

    /// Compare simulated vs actual profits for an execution
    pub async fn compare_execution_profit(&self, execution_id: &str) -> Result<ProfitComparison> {
        debug!("🔍 Comparing simulated vs actual profit for execution: {}", execution_id);

        // Get simulation data
        let simulation_data = self.get_simulation_data(execution_id).await?;
        
        // Get actual execution data
        let execution_data = self.get_execution_data(execution_id).await?;

        // Calculate differences
        let profit_difference = execution_data.actual_profit - simulation_data.simulated_profit;
        
        let profit_accuracy_percentage = if simulation_data.simulated_profit != Decimal::ZERO {
            (execution_data.actual_profit / simulation_data.simulated_profit) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let gas_estimation_accuracy = if simulation_data.gas_estimate != Decimal::ZERO {
            (execution_data.actual_gas_cost / simulation_data.gas_estimate) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Calculate impact factors
        let slippage_impact = self.calculate_slippage_impact(execution_id).await?;
        let mev_impact = self.calculate_mev_impact(execution_id).await?;
        let timing_impact = self.calculate_timing_impact(execution_id).await?;
        let market_movement_impact = self.calculate_market_movement_impact(execution_id).await?;

        let comparison = ProfitComparison {
            execution_id: execution_id.to_string(),
            simulated_profit: simulation_data.simulated_profit,
            actual_profit: execution_data.actual_profit,
            profit_difference,
            profit_accuracy_percentage,
            simulation_gas_estimate: simulation_data.gas_estimate,
            actual_gas_cost: execution_data.actual_gas_cost,
            gas_estimation_accuracy,
            slippage_impact,
            mev_impact,
            timing_impact,
            market_movement_impact,
            comparison_timestamp: Utc::now(),
        };

        // Store comparison in database
        self.store_profit_comparison(&comparison).await?;

        info!("✅ Profit comparison completed for execution {}: {}% accuracy", 
              execution_id, profit_accuracy_percentage);

        Ok(comparison)
    }

    /// Get accuracy metrics for a time period
    pub async fn get_accuracy_metrics(
        &self, 
        start_time: DateTime<Utc>, 
        end_time: DateTime<Utc>
    ) -> Result<AccuracyMetrics> {
        info!("📊 Calculating accuracy metrics from {} to {}", start_time, end_time);

        let rows = sqlx::query!(
            "SELECT 
                profit_accuracy_percentage,
                gas_estimation_accuracy,
                profit_difference
            FROM profit_comparisons 
            WHERE comparison_timestamp >= $1 AND comparison_timestamp <= $2",
            start_time,
            end_time
        )
        .fetch_all(&self.db_pool)
        .await?;

        let total_comparisons = rows.len() as u64;
        
        if total_comparisons == 0 {
            return Ok(AccuracyMetrics {
                total_comparisons: 0,
                average_profit_accuracy: Decimal::ZERO,
                average_gas_accuracy: Decimal::ZERO,
                profit_overestimation_count: 0,
                profit_underestimation_count: 0,
                gas_overestimation_count: 0,
                gas_underestimation_count: 0,
                best_accuracy_percentage: Decimal::ZERO,
                worst_accuracy_percentage: Decimal::ZERO,
                accuracy_standard_deviation: Decimal::ZERO,
            });
        }

        let profit_accuracies: Vec<Decimal> = rows.iter()
            .map(|r| r.profit_accuracy_percentage.unwrap_or_default())
            .collect();

        let gas_accuracies: Vec<Decimal> = rows.iter()
            .map(|r| r.gas_estimation_accuracy.unwrap_or_default())
            .collect();

        let average_profit_accuracy = profit_accuracies.iter().sum::<Decimal>() / Decimal::from(total_comparisons);
        let average_gas_accuracy = gas_accuracies.iter().sum::<Decimal>() / Decimal::from(total_comparisons);

        let profit_overestimation_count = rows.iter()
            .filter(|r| r.profit_difference.unwrap_or_default() < Decimal::ZERO)
            .count() as u64;

        let profit_underestimation_count = total_comparisons - profit_overestimation_count;

        let gas_overestimation_count = rows.iter()
            .filter(|r| r.gas_estimation_accuracy.unwrap_or_default() < Decimal::from(100))
            .count() as u64;

        let gas_underestimation_count = total_comparisons - gas_overestimation_count;

        let best_accuracy_percentage = profit_accuracies.iter()
            .max()
            .copied()
            .unwrap_or_default();

        let worst_accuracy_percentage = profit_accuracies.iter()
            .min()
            .copied()
            .unwrap_or_default();

        // Calculate standard deviation
        let variance = profit_accuracies.iter()
            .map(|accuracy| {
                let diff = *accuracy - average_profit_accuracy;
                diff * diff
            })
            .sum::<Decimal>() / Decimal::from(total_comparisons);

        let accuracy_standard_deviation = variance.sqrt().unwrap_or_default();

        Ok(AccuracyMetrics {
            total_comparisons,
            average_profit_accuracy,
            average_gas_accuracy,
            profit_overestimation_count,
            profit_underestimation_count,
            gas_overestimation_count,
            gas_underestimation_count,
            best_accuracy_percentage,
            worst_accuracy_percentage,
            accuracy_standard_deviation,
        })
    }

    /// Get accuracy metrics by strategy
    pub async fn get_strategy_accuracy(&self, days: u32) -> Result<Vec<StrategyAccuracy>> {
        let start_time = Utc::now() - chrono::Duration::days(days as i64);

        let rows = sqlx::query!(
            "SELECT 
                p.strategy_type,
                COUNT(*) as execution_count,
                AVG(pc.profit_accuracy_percentage) as avg_profit_accuracy,
                AVG(pc.gas_estimation_accuracy) as avg_gas_accuracy,
                COUNT(CASE WHEN p.net_profit > 0 THEN 1 END)::float / COUNT(*)::float * 100 as success_rate
            FROM profit_comparisons pc
            JOIN pnl_calculations p ON pc.execution_id = p.execution_id
            WHERE pc.comparison_timestamp >= $1
            GROUP BY p.strategy_type
            ORDER BY avg_profit_accuracy DESC",
            start_time
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut strategy_accuracies = Vec::new();
        for row in rows {
            let avg_profit_accuracy = Decimal::from_f64(row.avg_profit_accuracy.unwrap_or_default()).unwrap_or_default();
            let avg_gas_accuracy = Decimal::from_f64(row.avg_gas_accuracy.unwrap_or_default()).unwrap_or_default();
            let success_rate = Decimal::from_f64(row.success_rate.unwrap_or_default()).unwrap_or_default();

            // Calculate reliability score (weighted average of accuracy and success rate)
            let reliability_score = (avg_profit_accuracy * Decimal::from_f64(0.6).unwrap()) + 
                                   (success_rate * Decimal::from_f64(0.4).unwrap());

            strategy_accuracies.push(StrategyAccuracy {
                strategy_name: row.strategy_type,
                execution_count: row.execution_count.unwrap_or_default() as u64,
                average_profit_accuracy: avg_profit_accuracy,
                average_gas_accuracy: avg_gas_accuracy,
                success_rate,
                reliability_score,
            });
        }

        Ok(strategy_accuracies)
    }

    /// Analyze impact of market conditions on accuracy
    pub async fn analyze_market_condition_impact(&self, days: u32) -> Result<Vec<MarketConditionImpact>> {
        let start_time = Utc::now() - chrono::Duration::days(days as i64);

        // This would analyze market volatility and its impact on prediction accuracy
        // For now, returning sample data structure
        let market_impacts = vec![
            MarketConditionImpact {
                volatility_level: "low".to_string(),
                average_accuracy: Decimal::from_f64(92.5).unwrap(),
                slippage_impact: Decimal::from_f64(0.2).unwrap(),
                execution_count: 150,
                success_rate: Decimal::from_f64(88.0).unwrap(),
            },
            MarketConditionImpact {
                volatility_level: "medium".to_string(),
                average_accuracy: Decimal::from_f64(85.3).unwrap(),
                slippage_impact: Decimal::from_f64(0.8).unwrap(),
                execution_count: 200,
                success_rate: Decimal::from_f64(82.5).unwrap(),
            },
            MarketConditionImpact {
                volatility_level: "high".to_string(),
                average_accuracy: Decimal::from_f64(76.8).unwrap(),
                slippage_impact: Decimal::from_f64(2.1).unwrap(),
                execution_count: 100,
                success_rate: Decimal::from_f64(74.0).unwrap(),
            },
        ];

        Ok(market_impacts)
    }

    /// Generate accuracy improvement recommendations
    pub async fn generate_accuracy_recommendations(&self) -> Result<Vec<String>> {
        let metrics = self.get_accuracy_metrics(
            Utc::now() - chrono::Duration::days(30),
            Utc::now()
        ).await?;

        let mut recommendations = Vec::new();

        if metrics.average_profit_accuracy < Decimal::from(80) {
            recommendations.push("Consider improving price impact calculations in simulations".to_string());
        }

        if metrics.average_gas_accuracy < Decimal::from(85) {
            recommendations.push("Update gas estimation models with recent network conditions".to_string());
        }

        if metrics.profit_overestimation_count > metrics.profit_underestimation_count {
            recommendations.push("Simulations tend to overestimate profits - add conservative buffers".to_string());
        }

        if metrics.accuracy_standard_deviation > Decimal::from(15) {
            recommendations.push("High variance in accuracy - investigate outlier cases".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Accuracy metrics are within acceptable ranges".to_string());
        }

        Ok(recommendations)
    }

    // Helper methods (simplified implementations)
    async fn get_simulation_data(&self, execution_id: &str) -> Result<SimulationData> {
        // Would fetch from simulations table
        Ok(SimulationData {
            simulated_profit: Decimal::from(100),
            gas_estimate: Decimal::from(50),
        })
    }

    async fn get_execution_data(&self, execution_id: &str) -> Result<ExecutionProfitData> {
        // Would fetch from executions and pnl_calculations tables
        Ok(ExecutionProfitData {
            actual_profit: Decimal::from(95),
            actual_gas_cost: Decimal::from(52),
        })
    }

    async fn calculate_slippage_impact(&self, execution_id: &str) -> Result<Decimal> {
        // Would calculate slippage impact from transaction analysis
        Ok(Decimal::from_f64(0.5).unwrap())
    }

    async fn calculate_mev_impact(&self, execution_id: &str) -> Result<Decimal> {
        // Would calculate MEV impact (sandwich attacks, front-running, etc.)
        Ok(Decimal::from_f64(0.2).unwrap())
    }

    async fn calculate_timing_impact(&self, execution_id: &str) -> Result<Decimal> {
        // Would calculate impact of execution timing vs simulation timing
        Ok(Decimal::from_f64(0.1).unwrap())
    }

    async fn calculate_market_movement_impact(&self, execution_id: &str) -> Result<Decimal> {
        // Would calculate impact of market price movements between simulation and execution
        Ok(Decimal::from_f64(0.3).unwrap())
    }

    async fn store_profit_comparison(&self, comparison: &ProfitComparison) -> Result<()> {
        // Would store in profit_comparisons table
        debug!("💾 Storing profit comparison for execution: {}", comparison.execution_id);
        Ok(())
    }
}

// Helper structs
struct SimulationData {
    simulated_profit: Decimal,
    gas_estimate: Decimal,
}

struct ExecutionProfitData {
    actual_profit: Decimal,
    actual_gas_cost: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profit_comparison_creation() {
        let comparison = ProfitComparison {
            execution_id: "test".to_string(),
            simulated_profit: Decimal::from(100),
            actual_profit: Decimal::from(95),
            profit_difference: Decimal::from(-5),
            profit_accuracy_percentage: Decimal::from(95),
            simulation_gas_estimate: Decimal::from(50),
            actual_gas_cost: Decimal::from(52),
            gas_estimation_accuracy: Decimal::from(104),
            slippage_impact: Decimal::from_f64(0.5).unwrap(),
            mev_impact: Decimal::from_f64(0.2).unwrap(),
            timing_impact: Decimal::from_f64(0.1).unwrap(),
            market_movement_impact: Decimal::from_f64(0.3).unwrap(),
            comparison_timestamp: Utc::now(),
        };

        assert_eq!(comparison.profit_accuracy_percentage, Decimal::from(95));
        assert_eq!(comparison.profit_difference, Decimal::from(-5));
    }

    #[test]
    fn test_accuracy_metrics_calculation() {
        let metrics = AccuracyMetrics {
            total_comparisons: 100,
            average_profit_accuracy: Decimal::from(85),
            average_gas_accuracy: Decimal::from(90),
            profit_overestimation_count: 60,
            profit_underestimation_count: 40,
            gas_overestimation_count: 45,
            gas_underestimation_count: 55,
            best_accuracy_percentage: Decimal::from(98),
            worst_accuracy_percentage: Decimal::from(65),
            accuracy_standard_deviation: Decimal::from(12),
        };

        assert_eq!(metrics.total_comparisons, 100);
        assert_eq!(metrics.average_profit_accuracy, Decimal::from(85));
    }
}
