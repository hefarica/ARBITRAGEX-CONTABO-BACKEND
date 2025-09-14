use anyhow::Result;
use async_trait::async_trait;
use ethers::types::Transaction;

use crate::types::{MarketState, Opportunity};

pub mod arbitrage;
pub mod sandwich;
pub mod liquidation;

pub use arbitrage::ArbitrageStrategy;
pub use sandwich::SandwichStrategy;
pub use liquidation::LiquidationStrategy;

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    
    async fn analyze_transaction(
        &self,
        tx: &Transaction,
        market_state: &MarketState,
    ) -> Result<Option<Opportunity>>;
    
    async fn analyze_market_state(
        &self,
        market_state: &MarketState,
    ) -> Result<Vec<Opportunity>>;
}

