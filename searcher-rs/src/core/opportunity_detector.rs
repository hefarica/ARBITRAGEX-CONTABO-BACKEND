use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::strategies::Strategy;
use crate::types::{MarketState, Opportunity};
use crate::core::StateManager;

pub struct OpportunityDetector {
    provider: Arc<Provider<Http>>,
    state_manager: Arc<StateManager>,
    strategies: Arc<RwLock<Vec<Box<dyn Strategy>>>>,
    tx_opportunities: mpsc::Sender<Opportunity>,
}

impl OpportunityDetector {
    pub fn new(
        provider: Arc<Provider<Http>>,
        state_manager: Arc<StateManager>,
        tx_opportunities: mpsc::Sender<Opportunity>,
    ) -> Self {
        Self {
            provider,
            state_manager,
            strategies: Arc::new(RwLock::new(Vec::new())),
            tx_opportunities,
        }
    }

    pub async fn register_strategy(&self, strategy: Box<dyn Strategy>) {
        let mut strategies = self.strategies.write().await;
        info!("Registrando estrategia: {}", strategy.name());
        strategies.push(strategy);
    }

    pub async fn analyze_block(&self, block: &Block<Transaction>) -> Result<()> {
        let block_number = block.number.unwrap_or_default();
        debug!("Analizando bloque #{}", block_number);

        // Obtener estado actual del mercado
        let market_state = self.state_manager.get_market_state().await?;

        // Aplicar cada estrategia
        let strategies = self.strategies.read().await;
        for strategy in strategies.iter() {
            match self.apply_strategy(strategy.as_ref(), &market_state, block).await {
                Ok(opportunities) => {
                    for opp in opportunities {
                        if opp.expected_profit > U256::from(10_000_000_000_000_000u64) { // 0.01 ETH
                            info!(
                                "💎 Oportunidad encontrada: {} - Profit: {} ETH",
                                opp.strategy_type,
                                ethers::utils::format_ether(opp.expected_profit)
                            );
                            let _ = self.tx_opportunities.send(opp).await;
                        }
                    }
                }
                Err(e) => warn!("Error en estrategia {}: {}", strategy.name(), e),
            }
        }

        Ok(())
    }

    async fn apply_strategy(
        &self,
        strategy: &dyn Strategy,
        market_state: &MarketState,
        block: &Block<Transaction>,
    ) -> Result<Vec<Opportunity>> {
        let mut opportunities = Vec::new();

        // Analizar cada transacción del bloque
        for tx in &block.transactions {
            if let Some(opp) = strategy.analyze_transaction(tx, market_state).await? {
                opportunities.push(opp);
            }
        }

        // Análisis general del estado del mercado
        opportunities.extend(strategy.analyze_market_state(market_state).await?);

        Ok(opportunities)
    }

    pub async fn analyze_mempool_tx(&self, tx: &Transaction) -> Result<Vec<Opportunity>> {
        let mut all_opportunities = Vec::new();
        let market_state = self.state_manager.get_market_state().await?;
        
        let strategies = self.strategies.read().await;
        for strategy in strategies.iter() {
            if let Some(opp) = strategy.analyze_transaction(tx, &market_state).await? {
                if opp.expected_profit > U256::from(50_000_000_000_000_000u64) { // 0.05 ETH para mempool
                    all_opportunities.push(opp);
                }
            }
        }

        Ok(all_opportunities)
    }
}

