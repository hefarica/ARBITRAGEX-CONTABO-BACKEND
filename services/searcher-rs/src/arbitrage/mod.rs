//! # Arbitrage Detection and Execution Module
//! 
//! Core arbitrage logic for detecting and executing profitable opportunities
//! across multiple DEXs and protocols.

pub mod detector;
pub mod executor;
pub mod types;
pub mod flash_loans;

use anyhow::Result;
use ethers::prelude::*;
use std::collections::HashMap;
use tracing::{info, warn, error, debug};
use chrono::{DateTime, Utc};

pub use detector::*;
pub use executor::*;
pub use types::*;
pub use flash_loans::*;

/// Core arbitrage engine for ArbitrageX Supreme V3.0
pub struct ArbitrageEngine {
    detector: OpportunityDetector,
    executor: ArbitrageExecutor,
    flash_loan_manager: FlashLoanManager,
    active_opportunities: HashMap<String, ArbitrageOpportunity>,
    statistics: ArbitrageStatistics,
}

impl ArbitrageEngine {
    /// Create new arbitrage engine instance
    pub fn new(
        eth_client: Arc<Provider<Ws>>,
        anvil_client: Arc<Provider<Http>>,
        signer: LocalWallet,
    ) -> Self {
        let detector = OpportunityDetector::new(eth_client.clone());
        let executor = ArbitrageExecutor::new(eth_client.clone(), anvil_client.clone(), signer.clone());
        let flash_loan_manager = FlashLoanManager::new(eth_client.clone());

        Self {
            detector,
            executor,
            flash_loan_manager,
            active_opportunities: HashMap::new(),
            statistics: ArbitrageStatistics::default(),
        }
    }

    /// Detect arbitrage opportunities in real-time
    pub async fn detect_opportunities(&mut self) -> Result<Vec<ArbitrageOpportunity>> {
        debug!("Scanning for arbitrage opportunities...");
        
        let opportunities = self.detector.scan_opportunities().await?;
        
        // Filter profitable opportunities
        let profitable_opportunities: Vec<ArbitrageOpportunity> = opportunities
            .into_iter()
            .filter(|opp| opp.estimated_profit > 0.01) // Minimum 0.01 ETH profit
            .collect();

        if !profitable_opportunities.is_empty() {
            info!("Detected {} profitable arbitrage opportunities", profitable_opportunities.len());
        }

        Ok(profitable_opportunities)
    }

    /// Execute arbitrage opportunity
    pub async fn execute_opportunity(&mut self, opportunity: ArbitrageOpportunity) -> Result<ExecutionResult> {
        info!("Executing arbitrage opportunity: {} (Strategy: {})", 
              opportunity.id, opportunity.strategy_type);

        // Check if opportunity is still valid
        if !self.validate_opportunity(&opportunity).await? {
            warn!("Opportunity {} is no longer valid", opportunity.id);
            return Ok(ExecutionResult::Skipped("Invalid opportunity".to_string()));
        }

        // Check if we need flash loans
        let execution_result = if opportunity.requires_flash_loan {
            self.execute_with_flash_loan(opportunity).await?
        } else {
            self.execute_direct(opportunity).await?
        };

        // Update statistics
        self.update_statistics(&execution_result);

        Ok(execution_result)
    }

    /// Execute opportunity using flash loans
    async fn execute_with_flash_loan(&mut self, opportunity: ArbitrageOpportunity) -> Result<ExecutionResult> {
        debug!("Executing with flash loan for opportunity: {}", opportunity.id);
        
        // Select best flash loan provider
        let flash_loan_provider = self.flash_loan_manager
            .select_best_provider(&opportunity.token_in, opportunity.amount_in)
            .await?;

        info!("Selected flash loan provider: {}", flash_loan_provider.name);

        // Execute flash loan arbitrage
        let tx_hash = self.executor
            .execute_flash_loan_arbitrage(opportunity.clone(), flash_loan_provider)
            .await?;

        Ok(ExecutionResult::Success {
            tx_hash,
            profit: opportunity.estimated_profit,
            gas_used: 0, // Will be updated after transaction confirmation
            execution_time: chrono::Utc::now(),
        })
    }

    /// Execute opportunity directly (without flash loans)
    async fn execute_direct(&mut self, opportunity: ArbitrageOpportunity) -> Result<ExecutionResult> {
        debug!("Executing direct arbitrage for opportunity: {}", opportunity.id);

        let tx_hash = self.executor
            .execute_direct_arbitrage(opportunity.clone())
            .await?;

        Ok(ExecutionResult::Success {
            tx_hash,
            profit: opportunity.estimated_profit,
            gas_used: 0, // Will be updated after transaction confirmation
            execution_time: chrono::Utc::now(),
        })
    }

    /// Validate if opportunity is still profitable
    async fn validate_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<bool> {
        // Re-calculate current prices and profit potential
        let current_profit = self.detector
            .calculate_current_profit(opportunity)
            .await?;

        // Check if still profitable after gas costs
        Ok(current_profit > 0.005) // Minimum 0.005 ETH after gas
    }

    /// Update arbitrage statistics
    fn update_statistics(&mut self, result: &ExecutionResult) {
        self.statistics.total_attempts += 1;
        
        match result {
            ExecutionResult::Success { profit, .. } => {
                self.statistics.successful_arbitrages += 1;
                self.statistics.total_profit += profit;
            },
            ExecutionResult::Failed { .. } => {
                self.statistics.failed_arbitrages += 1;
            },
            ExecutionResult::Skipped(_) => {
                self.statistics.skipped_opportunities += 1;
            },
        }
    }

    /// Get current arbitrage statistics
    pub fn get_statistics(&self) -> &ArbitrageStatistics {
        &self.statistics
    }

    /// Get active opportunities
    pub fn get_active_opportunities(&self) -> Vec<&ArbitrageOpportunity> {
        self.active_opportunities.values().collect()
    }
}

/// Arbitrage execution result
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Success {
        tx_hash: H256,
        profit: f64,
        gas_used: u64,
        execution_time: DateTime<Utc>,
    },
    Failed {
        error: String,
        gas_used: u64,
    },
    Skipped(String),
}

/// Arbitrage statistics tracking
#[derive(Debug, Default, Clone)]
pub struct ArbitrageStatistics {
    pub total_attempts: u64,
    pub successful_arbitrages: u64,
    pub failed_arbitrages: u64,
    pub skipped_opportunities: u64,
    pub total_profit: f64,
    pub total_gas_spent: u64,
    pub average_execution_time: u64, // in milliseconds
}

impl ArbitrageStatistics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            (self.successful_arbitrages as f64) / (self.total_attempts as f64) * 100.0
        }
    }

    /// Calculate average profit per successful arbitrage
    pub fn average_profit(&self) -> f64 {
        if self.successful_arbitrages == 0 {
            0.0
        } else {
            self.total_profit / (self.successful_arbitrages as f64)
        }
    }
}