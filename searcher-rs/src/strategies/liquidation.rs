use anyhow::Result;
use async_trait::async_trait;
use ethers::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use tracing::debug;

use crate::strategies::Strategy;
use crate::types::{Config, MarketState, Opportunity};

pub struct LiquidationStrategy {
    config: Config,
    liquidation_threshold: f64,
}

impl LiquidationStrategy {
    pub fn new(config: Config) -> Self {
        Self {
            liquidation_threshold: 0.8, // 80% LTV
            config,
        }
    }

    async fn check_lending_positions(&self, market_state: &MarketState) -> Result<Vec<LiquidationOpportunity>> {
        let mut opportunities = Vec::new();

        // En producción, consultar contratos de lending reales
        // Por ahora, simulamos algunas posiciones
        debug!("Verificando posiciones de lending para liquidaciones");

        // Simulación de posiciones en riesgo
        if let Some(eth_price) = market_state.token_prices.get("WETH") {
            if eth_price.price_usd < 2000.0 {
                // Si ETH baja de $2000, algunas posiciones podrían estar en riesgo
                opportunities.push(LiquidationOpportunity {
                    protocol: "Aave".to_string(),
                    user: "0x1234...".to_string(),
                    collateral_token: "WETH".to_string(),
                    debt_token: "USDC".to_string(),
                    collateral_amount: U256::from(10_000_000_000_000_000_000u64), // 10 ETH
                    debt_amount: U256::from(15_000_000_000u64), // 15,000 USDC
                    liquidation_bonus: 0.05, // 5%
                });
            }
        }

        Ok(opportunities)
    }

    fn calculate_liquidation_profit(
        &self,
        opp: &LiquidationOpportunity,
        market_state: &MarketState,
    ) -> Result<U256> {
        let collateral_price = market_state
            .token_prices
            .get(&opp.collateral_token)
            .map(|p| p.price_usd)
            .unwrap_or(0.0);

        let debt_price = market_state
            .token_prices
            .get(&opp.debt_token)
            .map(|p| p.price_usd)
            .unwrap_or(1.0);

        // Valor del colateral en USD
        let collateral_value_usd = collateral_price * 
            (opp.collateral_amount.as_u64() as f64 / 1e18);

        // Valor de la deuda en USD
        let debt_value_usd = debt_price * 
            (opp.debt_amount.as_u64() as f64 / 1e6); // USDC tiene 6 decimales

        // Profit = (Colateral * (1 + bonus)) - Deuda
        let profit_usd = collateral_value_usd * (1.0 + opp.liquidation_bonus) - debt_value_usd;

        // Convertir a wei
        let profit_wei = U256::from((profit_usd * 1e18 / collateral_price) as u64);

        Ok(profit_wei)
    }
}

struct LiquidationOpportunity {
    protocol: String,
    user: String,
    collateral_token: String,
    debt_token: String,
    collateral_amount: U256,
    debt_amount: U256,
    liquidation_bonus: f64,
}

#[async_trait]
impl Strategy for LiquidationStrategy {
    fn name(&self) -> &str {
        "Liquidation"
    }

    async fn analyze_transaction(
        &self,
        _tx: &Transaction,
        _market_state: &MarketState,
    ) -> Result<Option<Opportunity>> {
        // Las liquidaciones se detectan por estado, no por transacciones
        Ok(None)
    }

    async fn analyze_market_state(
        &self,
        market_state: &MarketState,
    ) -> Result<Vec<Opportunity>> {
        let mut opportunities = Vec::new();

        let liquidation_opps = self.check_lending_positions(market_state).await?;

        for liq_opp in liquidation_opps {
            let profit = self.calculate_liquidation_profit(&liq_opp, market_state)?;
            let gas_cost = U256::from(500_000) * market_state.gas_price;

            if profit > gas_cost + U256::from(self.config.min_profit_wei) {
                let mut metadata = HashMap::new();
                metadata.insert("protocol".to_string(), serde_json::to_value(&liq_opp.protocol)?);
                metadata.insert("user".to_string(), serde_json::to_value(&liq_opp.user)?);
                metadata.insert(
                    "collateral_amount".to_string(),
                    serde_json::to_value(liq_opp.collateral_amount.to_string())?,
                );
                metadata.insert(
                    "debt_amount".to_string(),
                    serde_json::to_value(liq_opp.debt_amount.to_string())?,
                );
                metadata.insert(
                    "liquidation_bonus".to_string(),
                    serde_json::to_value(liq_opp.liquidation_bonus)?,
                );

                opportunities.push(Opportunity {
                    id: Uuid::new_v4().to_string(),
                    chain: "ethereum".to_string(),
                    strategy_type: "liquidation".to_string(),
                    tokens: vec![liq_opp.collateral_token, liq_opp.debt_token],
                    pools: vec![liq_opp.protocol],
                    expected_profit: profit - gas_cost,
                    gas_cost,
                    confidence: 0.9, // Alta confianza en liquidaciones
                    deadline: market_state.block_timestamp + 300, // 5 minutos
                    created_at: Utc::now(),
                    metadata,
                });
            }
        }

        debug!("Encontradas {} oportunidades de liquidación", opportunities.len());
        Ok(opportunities)
    }
}

