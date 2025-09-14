use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use tracing_subscriber;

mod core;
mod strategies;
mod types;
mod utils;

use crate::core::{OpportunityDetector, StateManager};
use crate::strategies::{ArbitrageStrategy, LiquidationStrategy, SandwichStrategy};
use crate::types::{Config, Opportunity};

#[tokio::main]
async fn main() -> Result<()> {
    // Inicializar logging
    tracing_subscriber::fmt()
        .with_env_filter("searcher_rs=debug,web3=info")
        .json()
        .init();

    info!("🚀 Iniciando ArbitrageX Searcher v3.0");

    // Cargar configuración
    let config = Config::from_env()?;
    
    // Conectar a nodo Ethereum
    let provider = Provider::<Http>::try_from(&config.rpc_url)?
        .interval(std::time::Duration::from_millis(config.block_time));
    let provider = Arc::new(provider);

    // Conectar a Redis
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = Arc::new(RwLock::new(redis_client.get_tokio_connection().await?));

    // Conectar a PostgreSQL
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    // Crear canales de comunicación
    let (tx_opportunities, mut rx_opportunities) = mpsc::channel::<Opportunity>(1000);
    let (tx_mempool, rx_mempool) = mpsc::channel(10000);

    // Inicializar state manager
    let state_manager = Arc::new(StateManager::new(
        provider.clone(),
        redis_conn.clone(),
        pool.clone(),
    ));

    // Inicializar detector de oportunidades
    let detector = OpportunityDetector::new(
        provider.clone(),
        state_manager.clone(),
        tx_opportunities.clone(),
    );

    // Registrar estrategias
    detector.register_strategy(Box::new(ArbitrageStrategy::new(config.clone())));
    detector.register_strategy(Box::new(SandwichStrategy::new(config.clone())));
    detector.register_strategy(Box::new(LiquidationStrategy::new(config.clone())));

    // Spawn task para monitorear mempool
    let mempool_monitor = tokio::spawn(monitor_mempool(
        provider.clone(),
        tx_mempool,
        config.clone(),
    ));

    // Spawn task para monitorear bloques
    let block_monitor = tokio::spawn(monitor_blocks(
        provider.clone(),
        detector,
        state_manager.clone(),
    ));

    // Spawn task para procesar oportunidades
    let opportunity_processor = tokio::spawn(process_opportunities(
        rx_opportunities,
        pool.clone(),
        redis_conn.clone(),
    ));

    // Esperar a que todos los tasks terminen
    tokio::select! {
        res = mempool_monitor => {
            error!("Mempool monitor terminó: {:?}", res);
        }
        res = block_monitor => {
            error!("Block monitor terminó: {:?}", res);
        }
        res = opportunity_processor => {
            error!("Opportunity processor terminó: {:?}", res);
        }
    }

    Ok(())
}

async fn monitor_mempool(
    provider: Arc<Provider<Http>>,
    tx: mpsc::Sender<Transaction>,
    config: Config,
) -> Result<()> {
    info!("🔍 Iniciando monitoreo de mempool");
    
    let ws_provider = Provider::<Ws>::connect(&config.ws_url).await?;
    let mut stream = ws_provider.subscribe_pending_txs().await?;

    while let Some(tx_hash) = stream.next().await {
        if let Ok(Some(tx)) = provider.get_transaction(tx_hash).await {
            if tx.value > U256::from(config.min_profit_wei) {
                let _ = tx.send(tx).await;
            }
        }
    }

    Ok(())
}

async fn monitor_blocks(
    provider: Arc<Provider<Http>>,
    detector: OpportunityDetector,
    state_manager: Arc<StateManager>,
) -> Result<()> {
    info!("📦 Iniciando monitoreo de bloques");
    
    let mut block_stream = provider.watch_blocks().await?;

    while let Some(block_hash) = block_stream.next().await {
        match provider.get_block_with_txs(block_hash).await {
            Ok(Some(block)) => {
                info!("Nuevo bloque: #{}", block.number.unwrap());
                
                // Actualizar state
                state_manager.update_block_data(&block).await?;
                
                // Detectar oportunidades
                detector.analyze_block(&block).await?;
            }
            Ok(None) => warn!("Bloque no encontrado: {:?}", block_hash),
            Err(e) => error!("Error obteniendo bloque: {}", e),
        }
    }

    Ok(())
}

async fn process_opportunities(
    mut rx: mpsc::Receiver<Opportunity>,
    pool: sqlx::PgPool,
    redis_conn: Arc<RwLock<redis::aio::Connection>>,
) -> Result<()> {
    info!("💰 Iniciando procesador de oportunidades");

    while let Some(opportunity) = rx.recv().await {
        info!(
            "Oportunidad detectada: {} - Profit: {} wei",
            opportunity.strategy_type, opportunity.expected_profit
        );

        // Guardar en PostgreSQL
        sqlx::query!(
            r#"
            INSERT INTO opportunities (chain, tokens, profit, strategy_type, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            opportunity.chain,
            &opportunity.tokens,
            opportunity.expected_profit as i64,
            opportunity.strategy_type,
            opportunity.created_at
        )
        .execute(&pool)
        .await?;

        // Publicar en Redis para selector-api
        let mut conn = redis_conn.write().await;
        redis::cmd("PUBLISH")
            .arg("opportunities")
            .arg(serde_json::to_string(&opportunity)?)
            .query_async::<_, ()>(&mut *conn)
            .await?;
    }

    Ok(())
}

