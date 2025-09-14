use anyhow::Result;
use ethers::prelude::*;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::types::{MarketState, PoolState, TokenPrice};

pub struct StateManager {
    provider: Arc<Provider<Http>>,
    redis_conn: Arc<RwLock<redis::aio::Connection>>,
    pool: sqlx::PgPool,
    market_state: Arc<RwLock<MarketState>>,
}

impl StateManager {
    pub fn new(
        provider: Arc<Provider<Http>>,
        redis_conn: Arc<RwLock<redis::aio::Connection>>,
        pool: sqlx::PgPool,
    ) -> Self {
        Self {
            provider,
            redis_conn,
            pool,
            market_state: Arc::new(RwLock::new(MarketState::default())),
        }
    }

    pub async fn get_market_state(&self) -> Result<MarketState> {
        let state = self.market_state.read().await;
        Ok(state.clone())
    }

    pub async fn update_block_data(&self, block: &Block<Transaction>) -> Result<()> {
        let block_number = block.number.unwrap_or_default().as_u64();
        let block_timestamp = block.timestamp.as_u64();
        
        info!("Actualizando state para bloque #{}", block_number);

        let mut state = self.market_state.write().await;
        state.block_number = block_number;
        state.block_timestamp = block_timestamp;
        state.gas_price = self.provider.get_gas_price().await?;

        // Actualizar precios de tokens desde Redis
        self.update_token_prices(&mut state).await?;

        // Actualizar estados de pools
        self.update_pool_states(&mut state).await?;

        // Guardar snapshot en Redis
        self.save_state_snapshot(&state).await?;

        Ok(())
    }

    async fn update_token_prices(&self, state: &mut MarketState) -> Result<()> {
        let mut conn = self.redis_conn.write().await;
        
        // Lista de tokens principales
        let tokens = vec![
            "WETH", "USDC", "USDT", "DAI", "WBTC", "LINK", "UNI", "AAVE"
        ];

        for token in tokens {
            let key = format!("price:{}", token);
            if let Ok(price) = conn.get::<_, String>(&key).await {
                if let Ok(price_value) = price.parse::<f64>() {
                    state.token_prices.insert(
                        token.to_string(),
                        TokenPrice {
                            symbol: token.to_string(),
                            price_usd: price_value,
                            last_update: state.block_timestamp,
                        },
                    );
                }
            }
        }

        debug!("Precios actualizados: {} tokens", state.token_prices.len());
        Ok(())
    }

    async fn update_pool_states(&self, state: &mut MarketState) -> Result<()> {
        // Consultar pools principales desde la base de datos
        let pools = sqlx::query!(
            r#"
            SELECT address, token0, token1, reserve0, reserve1, fee
            FROM pools
            WHERE active = true
            ORDER BY tvl_usd DESC
            LIMIT 100
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        state.pools.clear();
        for pool in pools {
            let pool_state = PoolState {
                address: pool.address,
                token0: pool.token0,
                token1: pool.token1,
                reserve0: U256::from_dec_str(&pool.reserve0.to_string())?,
                reserve1: U256::from_dec_str(&pool.reserve1.to_string())?,
                fee: pool.fee as u32,
                last_update: state.block_timestamp,
            };
            state.pools.insert(pool.address.clone(), pool_state);
        }

        debug!("Pools actualizados: {}", state.pools.len());
        Ok(())
    }

    async fn save_state_snapshot(&self, state: &MarketState) -> Result<()> {
        let mut conn = self.redis_conn.write().await;
        let key = format!("state:block:{}", state.block_number);
        let value = serde_json::to_string(state)?;
        
        // Guardar con TTL de 1 hora
        conn.set_ex(&key, value, 3600).await?;
        
        // Actualizar el último estado
        conn.set("state:latest", &key).await?;

        Ok(())
    }

    pub async fn get_pool_reserves(&self, pool_address: &str) -> Result<(U256, U256)> {
        let state = self.market_state.read().await;
        if let Some(pool) = state.pools.get(pool_address) {
            Ok((pool.reserve0, pool.reserve1))
        } else {
            // Si no está en cache, consultar on-chain
            self.fetch_pool_reserves_onchain(pool_address).await
        }
    }

    async fn fetch_pool_reserves_onchain(&self, pool_address: &str) -> Result<(U256, U256)> {
        // ABI mínimo para getReserves()
        let abi = r#"[{"constant":true,"inputs":[],"name":"getReserves","outputs":[{"name":"reserve0","type":"uint112"},{"name":"reserve1","type":"uint112"},{"name":"blockTimestampLast","type":"uint32"}],"payable":false,"stateMutability":"view","type":"function"}]"#;
        
        let pool_address = pool_address.parse::<Address>()?;
        let pool = Contract::from_json(self.provider.clone(), pool_address, abi.as_bytes())?;
        
        let result: (U256, U256, u32) = pool.method("getReserves", ())?.call().await?;
        Ok((result.0, result.1))
    }
}

