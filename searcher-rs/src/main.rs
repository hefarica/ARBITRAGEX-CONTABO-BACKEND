use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use tracing_subscriber;
use serde_json;
use reqwest;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod api;
mod core;
mod database;
mod strategies;
mod types;
mod utils;
mod config;
mod blockchain;
mod dex;
mod api;
mod database;

use crate::core::{OpportunityDetector, StateManager};
use crate::strategies::{ArbitrageStrategy, LiquidationStrategy, SandwichStrategy};
use crate::types::{Config, Opportunity, ChainConfig};
use crate::api::server::ApiServer;
use crate::database::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter("searcher_rs=debug,web3=info,ethers=warn")
        .json()
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🚀 Starting ArbitrageX Searcher v3.0 - MEV Engine Core");

    // Load configuration from environment
    let config = Config::from_env()?;
    info!("📋 Configuration loaded: chains={}, strategies={}", 
          config.chains.len(), config.strategies.len());
    
    // Initialize database connections
    let database = Database::new(&config.database_url).await?;
    info!("🗄️ Database connection established");
    
    // Initialize Redis connection
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let mut redis_conn = redis_client.get_tokio_connection().await?;
    info!("🔴 Redis connection established");
    
    // Test Redis connection
    let _: () = redis::cmd("PING").query_async(&mut redis_conn).await?;
    let redis_conn = Arc::new(RwLock::new(redis_conn));

    // Initialize blockchain providers for each chain
    let mut providers = HashMap::new();
    for (chain_id, chain_config) in &config.chains {
        let provider = Provider::<Http>::try_from(&chain_config.rpc_url)?
            .interval(Duration::from_millis(chain_config.block_time));
        
        // Test connection
        let block_number = provider.get_block_number().await?;
        info!("⛓️ Connected to chain {} at block {}", chain_id, block_number);
        
        providers.insert(*chain_id, Arc::new(provider));
    }

    // Create communication channels
    let (tx_opportunities, mut rx_opportunities) = mpsc::channel::<Opportunity>(1000);
    let (tx_mempool, rx_mempool) = mpsc::channel(10000);
    let (tx_execution, rx_execution) = mpsc::channel(100);

    // Initialize state manager with real data
    let state_manager = Arc::new(StateManager::new(
        providers.clone(),
        redis_conn.clone(),
        database.clone(),
    ).await?);

    // Initialize opportunity detector with real market data
    let detector = Arc::new(OpportunityDetector::new(
        providers.clone(),
        state_manager.clone(),
        tx_opportunities.clone(),
        config.clone(),
    ).await?);

    // Register MEV strategies with real parameters
    detector.register_strategy(Box::new(ArbitrageStrategy::new(
        config.clone(),
        providers.clone(),
    ).await?)).await?;
    
    detector.register_strategy(Box::new(SandwichStrategy::new(
        config.clone(),
        providers.clone(),
    ).await?)).await?;
    
    detector.register_strategy(Box::new(LiquidationStrategy::new(
        config.clone(),
        providers.clone(),
    ).await?)).await?;

    info!("🎯 Registered {} MEV strategies", detector.strategy_count().await);

    // Initialize API server for external communication
    let api_server = ApiServer::new(
        config.api_port,
        state_manager.clone(),
        detector.clone(),
        database.clone(),
    );

    // Start real-time mempool monitoring for each chain
    let mut mempool_handles = Vec::new();
    for (chain_id, provider) in &providers {
        let tx_mempool_clone = tx_mempool.clone();
        let provider_clone = provider.clone();
        let chain_id = *chain_id;
        
        let handle = tokio::spawn(async move {
            monitor_mempool(chain_id, provider_clone, tx_mempool_clone).await
        });
        mempool_handles.push(handle);
    }

    // Start opportunity detection engine
    let detection_handle = tokio::spawn(async move {
        detector.start_detection().await
    });

    // Start opportunity processing pipeline
    let processing_handle = tokio::spawn(async move {
        process_opportunities(
            rx_opportunities,
            tx_execution,
            state_manager.clone(),
            database.clone(),
        ).await
    });

    // Start execution engine
    let execution_handle = tokio::spawn(async move {
        execute_opportunities(
            rx_execution,
            providers.clone(),
            config.clone(),
        ).await
    });

    // Start API server
    let api_handle = tokio::spawn(async move {
        api_server.start().await
    });

    // Start health monitoring
    let health_handle = tokio::spawn(async move {
        monitor_system_health(
            providers.clone(),
            redis_conn.clone(),
            database.clone(),
        ).await
    });

    info!("✨ ArbitrageX MEV Engine fully initialized and running");
    info!("📊 Monitoring {} chains for MEV opportunities", providers.len());
    info!("🌐 API server listening on port {}", config.api_port);

    // Wait for all tasks to complete (they should run indefinitely)
    tokio::select! {
        result = detection_handle => {
            error!("Detection engine stopped: {:?}", result);
        }
        result = processing_handle => {
            error!("Processing pipeline stopped: {:?}", result);
        }
        result = execution_handle => {
            error!("Execution engine stopped: {:?}", result);
        }
        result = api_handle => {
            error!("API server stopped: {:?}", result);
        }
        result = health_handle => {
            error!("Health monitor stopped: {:?}", result);
        }
        _ = tokio::signal::ctrl_c() => {
            info!("🛑 Received shutdown signal, gracefully shutting down...");
        }
    }

    // Graceful shutdown
    info!("🔄 Performing graceful shutdown...");
    
    // Cancel all mempool monitoring tasks
    for handle in mempool_handles {
        handle.abort();
    }

    info!("✅ ArbitrageX MEV Engine shutdown complete");
    Ok(())
}

// Real-time mempool monitoring function
async fn monitor_mempool(
    chain_id: u64,
    provider: Arc<Provider<Http>>,
    tx_mempool: mpsc::Sender<(u64, Transaction)>,
) -> Result<()> {
    info!("🔍 Starting mempool monitoring for chain {}", chain_id);
    
    let mut stream = provider.watch_pending_transactions().await?;
    
    while let Some(tx_hash) = stream.next().await {
        if let Ok(Some(tx)) = provider.get_transaction(tx_hash).await {
            // Filter for relevant transactions (DEX interactions, large values, etc.)
            if is_relevant_transaction(&tx) {
                if let Err(e) = tx_mempool.send((chain_id, tx)).await {
                    warn!("Failed to send mempool transaction: {}", e);
                    break;
                }
            }
        }
    }
    
    error!("Mempool monitoring stopped for chain {}", chain_id);
    Ok(())
}

// Process detected opportunities
async fn process_opportunities(
    mut rx_opportunities: mpsc::Receiver<Opportunity>,
    tx_execution: mpsc::Sender<Opportunity>,
    state_manager: Arc<StateManager>,
    database: Arc<Database>,
) -> Result<()> {
    info!("⚙️ Starting opportunity processing pipeline");
    
    while let Some(opportunity) = rx_opportunities.recv().await {
        // Validate opportunity is still profitable
        if let Ok(is_valid) = state_manager.validate_opportunity(&opportunity).await {
            if is_valid {
                // Store in database
                if let Err(e) = database.store_opportunity(&opportunity).await {
                    error!("Failed to store opportunity: {}", e);
                    continue;
                }
                
                // Send for execution
                if let Err(e) = tx_execution.send(opportunity).await {
                    error!("Failed to send opportunity for execution: {}", e);
                    break;
                }
            }
        }
    }
    
    Ok(())
}

// Execute profitable opportunities
async fn execute_opportunities(
    mut rx_execution: mpsc::Receiver<Opportunity>,
    providers: HashMap<u64, Arc<Provider<Http>>>,
    config: Config,
) -> Result<()> {
    info!("🚀 Starting opportunity execution engine");
    
    while let Some(opportunity) = rx_execution.recv().await {
        if let Some(provider) = providers.get(&opportunity.chain_id) {
            // Execute the opportunity
            match execute_single_opportunity(&opportunity, provider.clone(), &config).await {
                Ok(tx_hash) => {
                    info!("✅ Executed opportunity {} with tx: {}", 
                          opportunity.id, tx_hash);
                }
                Err(e) => {
                    error!("Failed to execute opportunity {}: {}", 
                           opportunity.id, e);
                }
            }
        }
    }
    
    Ok(())
}

// Execute a single opportunity
async fn execute_single_opportunity(
    opportunity: &Opportunity,
    provider: Arc<Provider<Http>>,
    config: &Config,
) -> Result<H256> {
    // Implementation would depend on the specific opportunity type
    // This is a placeholder for the actual execution logic
    todo!("Implement specific opportunity execution")
}

// Check if a transaction is relevant for MEV
fn is_relevant_transaction(tx: &Transaction) -> bool {
    // Check if transaction interacts with known DEX contracts
    // Check transaction value
    // Check gas price
    tx.value > U256::from(10u64.pow(17)) // > 0.1 ETH
        || tx.gas_price.unwrap_or_default() > U256::from(50_000_000_000u64) // > 50 gwei
}

// Monitor system health
async fn monitor_system_health(
    providers: HashMap<u64, Arc<Provider<Http>>>,
    redis_conn: Arc<RwLock<redis::aio::Connection>>,
    database: Arc<Database>,
) -> Result<()> {
    info!("👨‍⚕️ Starting system health monitoring");
    
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        // Check blockchain connections
        for (chain_id, provider) in &providers {
            match provider.get_block_number().await {
                Ok(block) => {
                    info!("Chain {} healthy at block {}", chain_id, block);
                }
                Err(e) => {
                    error!("Chain {} unhealthy: {}", chain_id, e);
                }
            }
        }
        
        // Check Redis connection
        {
            let mut conn = redis_conn.write().await;
            match redis::cmd("PING").query_async::<_, String>(&mut *conn).await {
                Ok(_) => info!("Redis connection healthy"),
                Err(e) => error!("Redis connection unhealthy: {}", e),
            }
        }
        
        // Check database connection
        match database.health_check().await {
            Ok(_) => info!("Database connection healthy"),
            Err(e) => error!("Database connection unhealthy: {}", e),
        }
    }
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

