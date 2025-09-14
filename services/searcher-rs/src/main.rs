//! # ArbitrageX Supreme V3.0 - MEV Searcher Main Entry Point
//! 
//! High-performance MEV searcher binary for ArbitrageX Supreme V3.0.
//! Implements 20 different arbitrage strategies with sub-200ms latency optimization.

use anyhow::Result;
use arbitragex_searcher::{ArbitrageXSearcher, SearcherConfig};
use clap::{Arg, Command};
use std::env;
use tracing::{info, error, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing();

    info!("ArbitrageX Supreme V3.0 - MEV Searcher Starting...");

    // Parse command line arguments
    let matches = Command::new("ArbitrageX Searcher")
        .version("3.0.0")
        .author("ArbitrageX Development Team")
        .about("High-performance MEV searcher for DeFi arbitrage")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("HTTP server port")
                .default_value("3001")
        )
        .get_matches();

    // Load configuration
    let config = load_config(&matches)?;

    // Initialize and start the searcher
    let searcher = ArbitrageXSearcher::new(config).await?;
    
    info!("ArbitrageX Searcher initialized successfully");
    info!("Starting main execution loop...");
    
    // Start the searcher (this will run indefinitely)
    if let Err(e) = searcher.start().await {
        error!("Searcher encountered an error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// Initialize tracing/logging system
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(true)
        .json()
        .init();
}

/// Load configuration from environment variables and command line arguments
fn load_config(matches: &clap::ArgMatches) -> Result<SearcherConfig> {
    // Load from environment variables with defaults
    let config = SearcherConfig {
        eth_rpc_url: env::var("GETH_RPC_URL")
            .unwrap_or_else(|_| "http://localhost:8545".to_string()),
        
        eth_ws_url: env::var("GETH_WS_URL")
            .unwrap_or_else(|_| "ws://localhost:8546".to_string()),
        
        anvil_rpc_url: env::var("ANVIL_RPC_URL")
            .unwrap_or_else(|_| "http://localhost:8555".to_string()),
        
        database_url: env::var("POSTGRES_URL")
            .expect("POSTGRES_URL environment variable is required"),
        
        redis_url: env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        
        private_key: env::var("PRIVATE_KEY")
            .expect("PRIVATE_KEY environment variable is required"),
        
        max_gas_price: env::var("MAX_GAS_PRICE")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or(100),
        
        min_profit_threshold: env::var("MIN_PROFIT_THRESHOLD")
            .unwrap_or_else(|_| "0.01".to_string())
            .parse()
            .unwrap_or(0.01),
        
        flashbots_bundle_executor: env::var("FLASHBOTS_BUNDLE_EXECUTOR")
            .expect("FLASHBOTS_BUNDLE_EXECUTOR environment variable is required"),
        
        port: matches.get_one::<String>("port")
            .unwrap()
            .parse()
            .unwrap_or(3001),
    };

    info!("Configuration loaded successfully");
    info!("Ethereum RPC: {}", config.eth_rpc_url);
    info!("Ethereum WS: {}", config.eth_ws_url);
    info!("Anvil RPC: {}", config.anvil_rpc_url);
    info!("Server Port: {}", config.port);
    info!("Max Gas Price: {} Gwei", config.max_gas_price);
    info!("Min Profit Threshold: {} ETH", config.min_profit_threshold);

    Ok(config)
}