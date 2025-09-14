use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn, error};
use serde_json;
use reqwest;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::strategies::Strategy;
use crate::types::{MarketState, Opportunity, Config, DexPool, TokenPrice};
use crate::core::StateManager;

pub struct OpportunityDetector {
    providers: HashMap<u64, Arc<Provider<Http>>>,
    state_manager: Arc<StateManager>,
    strategies: Arc<RwLock<Vec<Box<dyn Strategy>>>>,
    tx_opportunities: mpsc::Sender<Opportunity>,
    config: Config,
    defi_llama_client: reqwest::Client,
    coingecko_client: reqwest::Client,
    dex_pools: Arc<RwLock<HashMap<String, DexPool>>>,
    token_prices: Arc<RwLock<HashMap<String, TokenPrice>>>,
}

impl OpportunityDetector {
    pub async fn new(
        providers: HashMap<u64, Arc<Provider<Http>>>,
        state_manager: Arc<StateManager>,
        tx_opportunities: mpsc::Sender<Opportunity>,
        config: Config,
    ) -> Result<Self> {
        let defi_llama_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("ArbitrageX/3.0")
            .build()?;
            
        let coingecko_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("ArbitrageX/3.0")
            .build()?;

        let detector = Self {
            providers,
            state_manager,
            strategies: Arc::new(RwLock::new(Vec::new())),
            tx_opportunities,
            config,
            defi_llama_client,
            coingecko_client,
            dex_pools: Arc::new(RwLock::new(HashMap::new())),
            token_prices: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize with real market data
        detector.initialize_market_data().await?;
        
        Ok(detector)
    }

    pub async fn register_strategy(&self, strategy: Box<dyn Strategy>) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        info!("🎯 Registering MEV strategy: {}", strategy.name());
        strategies.push(strategy);
        Ok(())
    }
    
    pub async fn strategy_count(&self) -> usize {
        self.strategies.read().await.len()
    }

    // Initialize market data from real sources
    async fn initialize_market_data(&self) -> Result<()> {
        info!("📊 Initializing real market data from DeFiLlama and CoinGecko...");
        
        // Fetch DEX pool data from DeFiLlama
        self.fetch_dex_pools().await?;
        
        // Fetch token prices from CoinGecko
        self.fetch_token_prices().await?;
        
        info!("✅ Market data initialization complete");
        Ok(())
    }
    
    // Fetch real DEX pool data from DeFiLlama
    async fn fetch_dex_pools(&self) -> Result<()> {
        let url = "https://api.llama.fi/pools";
        
        match self.defi_llama_client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let pools_data: serde_json::Value = response.json().await?;
                    
                    if let Some(pools) = pools_data["data"].as_array() {
                        let mut dex_pools = self.dex_pools.write().await;
                        
                        for pool in pools.iter().take(1000) { // Limit to top 1000 pools
                            if let Ok(dex_pool) = self.parse_dex_pool(pool) {
                                dex_pools.insert(dex_pool.address.clone(), dex_pool);
                            }
                        }
                        
                        info!("📈 Loaded {} DEX pools from DeFiLlama", dex_pools.len());
                    }
                } else {
                    warn!("Failed to fetch DEX pools: HTTP {}", response.status());
                }
            }
            Err(e) => {
                error!("Error fetching DEX pools: {}", e);
            }
        }
        
        Ok(())
    }
    
    // Fetch real token prices from CoinGecko
    async fn fetch_token_prices(&self) -> Result<()> {
        let api_key = self.config.coingecko_api_key.as_deref().unwrap_or("");
        let url = if api_key.is_empty() {
            "https://api.coingecko.com/api/v3/simple/price?ids=ethereum,bitcoin,binancecoin,matic-network,avalanche-2&vs_currencies=usd".to_string()
        } else {
            format!("https://pro-api.coingecko.com/api/v3/simple/price?ids=ethereum,bitcoin,binancecoin,matic-network,avalanche-2&vs_currencies=usd&x_cg_pro_api_key={}", api_key)
        };
        
        match self.coingecko_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let prices_data: serde_json::Value = response.json().await?;
                    let mut token_prices = self.token_prices.write().await;
                    
                    for (token_id, price_data) in prices_data.as_object().unwrap_or(&serde_json::Map::new()) {
                        if let Some(usd_price) = price_data["usd"].as_f64() {
                            let token_price = TokenPrice {
                                token_id: token_id.clone(),
                                price_usd: usd_price,
                                last_updated: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                            };
                            token_prices.insert(token_id.clone(), token_price);
                        }
                    }
                    
                    info!("💰 Loaded {} token prices from CoinGecko", token_prices.len());
                } else {
                    warn!("Failed to fetch token prices: HTTP {}", response.status());
                }
            }
            Err(e) => {
                error!("Error fetching token prices: {}", e);
            }
        }
        
        Ok(())
    }
    
    // Parse DeFiLlama pool data into our format
    fn parse_dex_pool(&self, pool_data: &serde_json::Value) -> Result<DexPool> {
        let pool = DexPool {
            address: pool_data["pool"].as_str().unwrap_or("").to_string(),
            chain: pool_data["chain"].as_str().unwrap_or("").to_string(),
            project: pool_data["project"].as_str().unwrap_or("").to_string(),
            symbol: pool_data["symbol"].as_str().unwrap_or("").to_string(),
            tvl_usd: pool_data["tvlUsd"].as_f64().unwrap_or(0.0),
            apy: pool_data["apy"].as_f64().unwrap_or(0.0),
            volume_usd_1d: pool_data["volumeUsd1d"].as_f64().unwrap_or(0.0),
            volume_usd_7d: pool_data["volumeUsd7d"].as_f64().unwrap_or(0.0),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        };
        
        Ok(pool)
    }

    // Start the main detection loop
    pub async fn start_detection(&self) -> Result<()> {
        info!("🔍 Starting MEV opportunity detection engine...");
        
        // Start price update task
        let price_update_handle = self.start_price_updates();
        
        // Start pool update task
        let pool_update_handle = self.start_pool_updates();
        
        // Start block monitoring for each chain
        let mut block_handles = Vec::new();
        for (chain_id, provider) in &self.providers {
            let detector_clone = Arc::new(self.clone_detector());
            let provider_clone = provider.clone();
            let chain_id = *chain_id;
            
            let handle = tokio::spawn(async move {
                detector_clone.monitor_chain_blocks(chain_id, provider_clone).await
            });
            block_handles.push(handle);
        }
        
        // Wait for all tasks
        tokio::select! {
            _ = price_update_handle => {
                error!("Price update task stopped");
            }
            _ = pool_update_handle => {
                error!("Pool update task stopped");
            }
            result = futures::future::try_join_all(block_handles) => {
                error!("Block monitoring stopped: {:?}", result);
            }
        }
        
        Ok(())
    }
    
    // Monitor blocks for a specific chain
    async fn monitor_chain_blocks(&self, chain_id: u64, provider: Arc<Provider<Http>>) -> Result<()> {
        info!("⛓️ Starting block monitoring for chain {}", chain_id);
        
        let mut stream = provider.watch_blocks().await?;
        
        while let Some(block_hash) = stream.next().await {
            if let Ok(Some(block)) = provider.get_block_with_txs(block_hash).await {
                if let Err(e) = self.analyze_block(chain_id, &block).await {
                    error!("Error analyzing block {}: {}", block.number.unwrap_or_default(), e);
                }
            }
        }
        
        Ok(())
    }

    pub async fn analyze_block(&self, chain_id: u64, block: &Block<Transaction>) -> Result<()> {
        let block_number = block.number.unwrap_or_default();
        debug!("🔍 Analyzing block #{} on chain {}", block_number, chain_id);

        // Get current market state with real data
        let market_state = self.get_real_market_state(chain_id).await?;

        // Apply each registered strategy
        let strategies = self.strategies.read().await;
        for strategy in strategies.iter() {
            match self.apply_strategy(strategy.as_ref(), &market_state, block, chain_id).await {
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

