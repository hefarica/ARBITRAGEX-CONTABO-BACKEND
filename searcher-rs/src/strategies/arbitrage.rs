use anyhow::Result;
use async_trait::async_trait;
use ethers::prelude::*;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use tracing::{debug, trace};

use crate::strategies::Strategy;
use crate::types::{Config, MarketState, Opportunity, PoolState};

pub struct ArbitrageStrategy {
    config: Config,
    min_profit_threshold: U256,
}

impl ArbitrageStrategy {
    pub fn new(config: Config) -> Self {
        Self {
            min_profit_threshold: U256::from(config.min_profit_wei),
            config,
        }
    }

    fn calculate_arbitrage_profit(
        &self,
        pool_a: &PoolState,
        pool_b: &PoolState,
        amount_in: U256,
    ) -> Result<Option<(U256, Vec<String>)>> {
        // Calcular output del primer pool
        let amount_out_a = self.get_amount_out(
            amount_in,
            pool_a.reserve0,
            pool_a.reserve1,
            pool_a.fee,
        )?;

        // Calcular output del segundo pool (swap inverso)
        let amount_out_b = self.get_amount_out(
            amount_out_a,
            pool_b.reserve1,
            pool_b.reserve0,
            pool_b.fee,
        )?;

        // Calcular profit
        if amount_out_b > amount_in {
            let profit = amount_out_b - amount_in;
            let path = vec![
                pool_a.token0.clone(),
                pool_a.token1.clone(),
                pool_b.token0.clone(),
            ];
            Ok(Some((profit, path)))
        } else {
            Ok(None)
        }
    }

    fn get_amount_out(
        &self,
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
        fee: u32,
    ) -> Result<U256> {
        let amount_in_with_fee = amount_in * U256::from(10000 - fee);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * U256::from(10000) + amount_in_with_fee;
        Ok(numerator / denominator)
    }

    fn find_optimal_amount(&self, pool_a: &PoolState, pool_b: &PoolState) -> U256 {
        // Implementación simplificada - en producción usar búsqueda binaria
        let test_amounts = vec![
            U256::from(1_000_000_000_000_000_000u64), // 1 ETH
            U256::from(5_000_000_000_000_000_000u64), // 5 ETH
            U256::from(10_000_000_000_000_000_000u64), // 10 ETH
            U256::from(50_000_000_000_000_000_000u64), // 50 ETH
        ];

        let mut best_amount = U256::zero();
        let mut best_profit = U256::zero();

        for amount in test_amounts {
            if let Ok(Some((profit, _))) = self.calculate_arbitrage_profit(pool_a, pool_b, amount) {
                if profit > best_profit {
                    best_profit = profit;
                    best_amount = amount;
                }
            }
        }

        best_amount
    }
}

#[async_trait]
impl Strategy for ArbitrageStrategy {
    fn name(&self) -> &str {
        "Arbitrage"
    }

    async fn analyze_transaction(
        &self,
        tx: &Transaction,
        market_state: &MarketState,
    ) -> Result<Option<Opportunity>> {
        // Analizar si la transacción afecta pools que podemos arbitrar
        trace!("Analizando tx {:?} para arbitraje", tx.hash);
        
        // Por ahora, retornamos None - en producción analizar el input data
        Ok(None)
    }

    async fn analyze_market_state(
        &self,
        market_state: &MarketState,
    ) -> Result<Vec<Opportunity>> {
        let mut opportunities = Vec::new();
        
        debug!("Buscando oportunidades de arbitraje en {} pools", market_state.pools.len());

        // Comparar todos los pares de pools
        let pools: Vec<&PoolState> = market_state.pools.values().collect();
        
        for i in 0..pools.len() {
            for j in i + 1..pools.len() {
                let pool_a = pools[i];
                let pool_b = pools[j];

                // Verificar si comparten tokens
                if (pool_a.token0 == pool_b.token0 && pool_a.token1 == pool_b.token1) ||
                   (pool_a.token0 == pool_b.token1 && pool_a.token1 == pool_b.token0) {
                    
                    // Encontrar cantidad óptima
                    let optimal_amount = self.find_optimal_amount(pool_a, pool_b);
                    
                    if let Ok(Some((profit, path))) = self.calculate_arbitrage_profit(
                        pool_a,
                        pool_b,
                        optimal_amount,
                    ) {
                        if profit > self.min_profit_threshold {
                            let gas_cost = U256::from(300_000) * market_state.gas_price;
                            let net_profit = profit.saturating_sub(gas_cost);
                            
                            if net_profit > U256::zero() {
                                let mut metadata = HashMap::new();
                                metadata.insert(
                                    "optimal_amount".to_string(),
                                    serde_json::to_value(optimal_amount.to_string())?,
                                );
                                metadata.insert(
                                    "pool_a".to_string(),
                                    serde_json::to_value(&pool_a.address)?,
                                );
                                metadata.insert(
                                    "pool_b".to_string(),
                                    serde_json::to_value(&pool_b.address)?,
                                );

                                opportunities.push(Opportunity {
                                    id: Uuid::new_v4().to_string(),
                                    chain: "ethereum".to_string(),
                                    strategy_type: "arbitrage".to_string(),
                                    tokens: path,
                                    pools: vec![pool_a.address.clone(), pool_b.address.clone()],
                                    expected_profit: net_profit,
                                    gas_cost,
                                    confidence: 0.85,
                                    deadline: market_state.block_timestamp + 120, // 2 minutos
                                    created_at: Utc::now(),
                                    metadata,
                                });
                            }
                        }
                    }
                }
            }
        }

        debug!("Encontradas {} oportunidades de arbitraje", opportunities.len());
        Ok(opportunities)
    }
}

