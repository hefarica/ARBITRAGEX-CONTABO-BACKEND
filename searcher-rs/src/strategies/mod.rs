use anyhow::Result;
use async_trait::async_trait;
use ethers::types::{Transaction, Block};
use std::collections::HashMap;

use crate::types::{MarketState, Opportunity, DexPool, TokenPrice};

pub mod arbitrage;
pub mod sandwich;
pub mod liquidation;
pub mod triangular;
pub mod flash_loan;

pub use arbitrage::ArbitrageStrategy;
pub use sandwich::SandwichStrategy;
pub use liquidation::LiquidationStrategy;
pub use triangular::TriangularArbitrageStrategy;
pub use flash_loan::FlashLoanArbitrageStrategy;

/// Core Strategy trait for MEV opportunity detection
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Get the strategy name
    fn name(&self) -> String;
    
    /// Analyze a block for opportunities
    async fn analyze_opportunity(
        &self,
        chain_id: u64,
        market_state: &MarketState,
        block: &Block<Transaction>,
        dex_pools: &HashMap<String, DexPool>,
        token_prices: &HashMap<String, TokenPrice>,
    ) -> Result<Vec<Opportunity>>;
    
    /// Validate an opportunity is still profitable
    async fn validate_opportunity(&self, opportunity: &Opportunity) -> Result<bool>;
    
    /// Estimate gas cost for executing the opportunity
    async fn estimate_gas(&self, opportunity: &Opportunity) -> Result<ethers::types::U256>;
}

