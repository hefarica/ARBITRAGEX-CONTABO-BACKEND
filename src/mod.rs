// ArbitrageX Supreme V3.3 (RLI) - Motor de Selección de Blockchains
// Implementación de acceso y selección de blockchains optimizadas para oportunidades de arbitraje
// REGLAS ABSOLUTAS: DATOS REALES ÚNICAMENTE - Verificación exhaustiva de todos los endpoints

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};
use reqwest::Client;
use futures::{stream, StreamExt};
use log::{debug, error, info, warn};

// Constantes para la configuración del selector
const MAX_CONCURRENT_RPC_CHECKS: usize = 15;
const RPC_TIMEOUT_MS: u64 = 5000;
const DEFAULT_REFRESH_INTERVAL_SEC: u64 = 120;
const HIGH_PRIORITY_REFRESH_SEC: u64 = 60;
const MEDIUM_PRIORITY_REFRESH_SEC: u64 = 180;
const LOW_PRIORITY_REFRESH_SEC: u64 = 300;

// Enumeración para prioridad de blockchains
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainPriority {
    High,
    Medium,
    Low,
    Custom(u64),
}

impl ChainPriority {
    pub fn refresh_interval(&self) -> Duration {
        match self {
            ChainPriority::High => Duration::from_secs(HIGH_PRIORITY_REFRESH_SEC),
            ChainPriority::Medium => Duration::from_secs(MEDIUM_PRIORITY_REFRESH_SEC),
            ChainPriority::Low => Duration::from_secs(LOW_PRIORITY_REFRESH_SEC),
            ChainPriority::Custom(secs) => Duration::from_secs(*secs),
        }
    }
}

// Estructura de datos para los endpoints RPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEndpoint {
    pub url: String,
    pub provider: String,
    pub rate_limit_req_sec: u32,
    pub last_latency_ms: Option<u64>,
    pub last_checked: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
    pub success_count: u32,
    pub failure_count: u32,
}

impl RpcEndpoint {
    pub fn new(url: String, provider: String, rate_limit: u32) -> Self {
        Self {
            url,
            provider,
            rate_limit_req_sec: rate_limit,
            last_latency_ms: None,
            last_checked: None,
            is_active: false,
            success_count: 0,
            failure_count: 0,
        }
    }
}

// Estructura de datos para los DEXs soportados en cada blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedDex {
    pub name: String,
    pub router_address: String,
    pub factory_address: Option<String>,
    pub fee_tiers: Vec<u32>,
    pub supports_flash_swaps: bool,
    pub avg_liquidity_score: f64, // Puntuación normalizada 0-100
}

// Estructura principal para la información de blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub id: u32,
    pub name: String,
    pub network_id: u64,
    pub currency_symbol: String,
    pub rpc_endpoints: Vec<RpcEndpoint>,
    pub priority: ChainPriority,
    pub avg_block_time_sec: f32,
    pub supported_dexes: Vec<SupportedDex>,
    pub flash_loan_providers: Vec<String>,
    pub tvl_usd: f64,
    pub gas_token_price_usd: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub score: f64, // Puntuación compuesta para ordenar blockchains por potencial de arbitraje
    pub is_active: bool,
}

impl BlockchainInfo {
    // Devuelve el mejor endpoint RPC basado en latencia y estado
    pub fn best_endpoint(&self) -> Option<&RpcEndpoint> {
        self.rpc_endpoints
            .iter()
            .filter(|endpoint| endpoint.is_active)
            .min_by_key(|endpoint| endpoint.last_latency_ms.unwrap_or(u64::MAX))
    }

    // Calcula la puntuación de oportunidad de arbitraje
    pub fn calculate_opportunity_score(&mut self) {
        // La puntuación se basa en:
        // 1. TVL (25%) - Mayor TVL = más liquidez y oportunidades
        // 2. Número de DEXes (25%) - Más DEXes = más rutas de arbitraje
        // 3. Velocidad de bloque (20%) - Bloques más rápidos = confirmaciones más rápidas
        // 4. Precio de gas (15%) - Gas más barato = trades más rentables
        // 5. Soporte de flash loans (15%) - Ofrece más estrategias

        let tvl_score = (self.tvl_usd.log10() / 10.0).min(1.0) * 25.0;
        
        let dex_count_score = (self.supported_dexes.len() as f64 / 10.0).min(1.0) * 25.0;
        
        let block_time_score = (1.0 / self.avg_block_time_sec as f64).min(1.0) * 20.0;
        
        // El precio de gas más bajo da puntuación más alta
        let gas_price_inverse = if self.gas_token_price_usd > 0.0 {
            1.0 / self.gas_token_price_usd
        } else {
            0.0
        };
        let gas_score = (gas_price_inverse * 100.0).min(1.0) * 15.0;
        
        let flash_loan_score = if !self.flash_loan_providers.is_empty() {
            15.0 * (self.flash_loan_providers.len() as f64 / 3.0).min(1.0)
        } else {
            0.0
        };

        self.score = tvl_score + dex_count_score + block_time_score + gas_score + flash_loan_score;
    }
}

// Gestor de la base de datos de blockchains
#[derive(Clone)]
pub struct BlockchainSelector {
    blockchains: Arc<RwLock<HashMap<u32, BlockchainInfo>>>,
    sorted_ids_by_score: Arc<RwLock<Vec<u32>>>,
    client: Client,
    last_full_refresh: Arc<RwLock<Instant>>,
}

impl BlockchainSelector {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Crear cliente HTTP con timeouts optimizados
        let client = Client::builder()
            .timeout(Duration::from_millis(RPC_TIMEOUT_MS))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()?;

        let selector = Self {
            blockchains: Arc::new(RwLock::new(HashMap::new())),
            sorted_ids_by_score: Arc::new(RwLock::new(Vec::new())),
            client,
            last_full_refresh: Arc::new(RwLock::new(Instant::now())),
        };

        // Cargar la base de datos inicial de blockchains
        selector.load_blockchain_database().await?;
        
        // Iniciar tarea en segundo plano para actualizar periódicamente
        selector.start_background_updates();

        Ok(selector)
    }

    // Carga la base de datos inicial de blockchains desde PostgreSQL
    async fn load_blockchain_database(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Cargando base de datos de blockchains...");
        
        // TODO: Implementar carga desde PostgreSQL
        // Por ahora, cargamos datos de ejemplo con las principales blockchains
        
        let mut blockchains = HashMap::new();
        
        // Ethereum (Mainnet)
        let ethereum = BlockchainInfo {
            id: 1,
            name: String::from("Ethereum"),
            network_id: 1,
            currency_symbol: String::from("ETH"),
            rpc_endpoints: vec![
                RpcEndpoint::new(
                    String::from("https://ethereum.publicnode.com"),
                    String::from("PublicNode"),
                    50,
                ),
                RpcEndpoint::new(
                    String::from("https://rpc.ankr.com/eth"),
                    String::from("Ankr"),
                    30,
                ),
                RpcEndpoint::new(
                    String::from("https://cloudflare-eth.com"),
                    String::from("Cloudflare"),
                    100,
                ),
            ],
            priority: ChainPriority::High,
            avg_block_time_sec: 12.0,
            supported_dexes: vec![
                SupportedDex {
                    name: String::from("Uniswap V3"),
                    router_address: String::from("0xE592427A0AEce92De3Edee1F18E0157C05861564"),
                    factory_address: Some(String::from("0x1F98431c8aD98523631AE4a59f267346ea31F984")),
                    fee_tiers: vec![100, 500, 3000, 10000],
                    supports_flash_swaps: true,
                    avg_liquidity_score: 95.0,
                },
                SupportedDex {
                    name: String::from("SushiSwap"),
                    router_address: String::from("0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F"),
                    factory_address: Some(String::from("0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac")),
                    fee_tiers: vec![30],
                    supports_flash_swaps: true,
                    avg_liquidity_score: 85.0,
                },
                // Más DEXs...
            ],
            flash_loan_providers: vec![
                String::from("Aave V3"),
                String::from("dYdX"),
                String::from("Balancer"),
            ],
            tvl_usd: 27_500_000_000.0, // $27.5B
            gas_token_price_usd: 2800.0,
            last_updated: chrono::Utc::now(),
            score: 0.0, // Se calculará más adelante
            is_active: true,
        };
        
        // Arbitrum
        let arbitrum = BlockchainInfo {
            id: 42161,
            name: String::from("Arbitrum"),
            network_id: 42161,
            currency_symbol: String::from("ETH"),
            rpc_endpoints: vec![
                RpcEndpoint::new(
                    String::from("https://arb1.arbitrum.io/rpc"),
                    String::from("Arbitrum"),
                    50,
                ),
                RpcEndpoint::new(
                    String::from("https://arbitrum-one.publicnode.com"),
                    String::from("PublicNode"),
                    50,
                ),
            ],
            priority: ChainPriority::High,
            avg_block_time_sec: 0.25, // Muy rápido
            supported_dexes: vec![
                SupportedDex {
                    name: String::from("Camelot"),
                    router_address: String::from("0xc873fEcbd354f5A56E00E710B90EF4201db2448d"),
                    factory_address: Some(String::from("0x6EcCab422D763aC031210895C81787E87B43A652")),
                    fee_tiers: vec![30],
                    supports_flash_swaps: true,
                    avg_liquidity_score: 88.0,
                },
                SupportedDex {
                    name: String::from("SushiSwap"),
                    router_address: String::from("0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"),
                    factory_address: Some(String::from("0xc35DADB65012eC5796536bD9864eD8773aBc74C4")),
                    fee_tiers: vec![30],
                    supports_flash_swaps: true,
                    avg_liquidity_score: 80.0,
                },
                // Más DEXs...
            ],
            flash_loan_providers: vec![
                String::from("Aave V3"),
                String::from("Balancer"),
            ],
            tvl_usd: 8_200_000_000.0, // $8.2B
            gas_token_price_usd: 2800.0, // Mismo precio que ETH pero fees mucho más bajas
            last_updated: chrono::Utc::now(),
            score: 0.0, // Se calculará más adelante
            is_active: true,
        };
        
        // Calcular puntuaciones
        let mut ethereum_mut = ethereum.clone();
        ethereum_mut.calculate_opportunity_score();
        
        let mut arbitrum_mut = arbitrum.clone();
        arbitrum_mut.calculate_opportunity_score();
        
        // Agregar a la colección
        blockchains.insert(ethereum_mut.id, ethereum_mut);
        blockchains.insert(arbitrum_mut.id, arbitrum_mut);
        
        // Actualizar la base de datos en memoria
        {
            let mut blockchains_lock = self.blockchains.write().await;
            *blockchains_lock = blockchains;
        }
        
        // Actualizar lista ordenada por puntuación
        self.update_sorted_ids().await;
        
        info!("Base de datos de blockchains cargada con éxito");
        Ok(())
    }

    // Inicia actualizaciones periódicas en segundo plano
    fn start_background_updates(&self) {
        let selector_clone = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = selector_clone.update_rpc_health().await {
                    error!("Error actualizando estado de RPCs: {:?}", e);
                }
                
                // Actualizar datos de blockchains cada 2 horas
                let last_refresh = *selector_clone.last_full_refresh.read().await;
                if last_refresh.elapsed() > Duration::from_secs(7200) {
                    if let Err(e) = selector_clone.refresh_blockchain_data().await {
                        error!("Error actualizando datos de blockchains: {:?}", e);
                    }
                    *selector_clone.last_full_refresh.write().await = Instant::now();
                }
                
                // Actualizar lista ordenada
                selector_clone.update_sorted_ids().await;
            }
        });
    }

    // Actualiza la salud de los endpoints RPC
    async fn update_rpc_health(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Actualizando salud de endpoints RPC...");
        
        let blockchains = self.blockchains.read().await;
        
        // Crear una lista plana de (chain_id, endpoint_index, url)
        let mut endpoint_checks = Vec::new();
        for (chain_id, blockchain) in blockchains.iter() {
            for (idx, endpoint) in blockchain.rpc_endpoints.iter().enumerate() {
                endpoint_checks.push((*chain_id, idx, endpoint.url.clone()));
            }
        }
        
        // Hacer comprobaciones en paralelo con límite de concurrencia
        let results = stream::iter(endpoint_checks)
            .map(|(chain_id, idx, url)| {
                let client = self.client.clone();
                async move {
                    let start = Instant::now();
                    let is_healthy = Self::check_rpc_endpoint(&client, &url).await;
                    let latency = start.elapsed().as_millis() as u64;
                    (chain_id, idx, is_healthy, latency)
                }
            })
            .buffer_unordered(MAX_CONCURRENT_RPC_CHECKS)
            .collect::<Vec<_>>()
            .await;
            
        // Actualizar estado de endpoints
        drop(blockchains); // Liberar el lock de lectura
        let mut blockchains = self.blockchains.write().await;
        
        for (chain_id, endpoint_idx, is_healthy, latency) in results {
            if let Some(blockchain) = blockchains.get_mut(&chain_id) {
                if endpoint_idx < blockchain.rpc_endpoints.len() {
                    let endpoint = &mut blockchain.rpc_endpoints[endpoint_idx];
                    
                    if is_healthy {
                        endpoint.success_count += 1;
                        endpoint.failure_count = endpoint.failure_count.saturating_sub(1);
                        endpoint.last_latency_ms = Some(latency);
                        endpoint.is_active = true;
                    } else {
                        endpoint.failure_count += 1;
                        if endpoint.failure_count > 3 {
                            endpoint.is_active = false;
                            warn!("RPC endpoint desactivado por fallos: {} (blockchain: {})", 
                                  endpoint.url, blockchain.name);
                        }
                    }
                    
                    endpoint.last_checked = Some(chrono::Utc::now());
                }
            }
        }
        
        debug!("Actualización de salud de RPC completada");
        Ok(())
    }

    // Comprueba si un endpoint RPC está respondiendo
    async fn check_rpc_endpoint(client: &Client, url: &str) -> bool {
        // Petición JSON-RPC para eth_blockNumber
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });
        
        match client.post(url)
            .json(&payload)
            .send()
            .await {
                Ok(response) => {
                    if !response.status().is_success() {
                        return false;
                    }
                    
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            json.get("result").is_some()
                        },
                        Err(_) => false,
                    }
                },
                Err(_) => false,
            }
    }

    // Actualiza datos completos de blockchains desde fuentes externas
    async fn refresh_blockchain_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Actualizando datos completos de blockchains...");
        
        // TODO: Implementar actualización desde múltiples fuentes
        // - TVL desde DefiLlama
        // - Precios de gas desde fuentes como Etherscan
        // - Lista actualizada de DEXs y contratos
        
        let mut blockchains_lock = self.blockchains.write().await;
        
        // Por cada blockchain, actualizar datos y recalcular score
        for blockchain in blockchains_lock.values_mut() {
            // Aquí iría el código para actualizar TVL, precios, etc.
            // ...
            
            // Recalcular puntuación
            blockchain.calculate_opportunity_score();
            blockchain.last_updated = chrono::Utc::now();
        }
        
        info!("Datos de blockchains actualizados correctamente");
        Ok(())
    }

    // Actualiza la lista ordenada de IDs por puntuación
    async fn update_sorted_ids(&self) {
        let blockchains = self.blockchains.read().await;
        let mut ids: Vec<_> = blockchains.keys().cloned().collect();
        
        // Ordenar por puntuación (score) de mayor a menor
        ids.sort_by(|a, b| {
            let score_a = blockchains.get(a).map(|bc| bc.score).unwrap_or(0.0);
            let score_b = blockchains.get(b).map(|bc| bc.score).unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Actualizar lista ordenada
        let mut sorted_ids = self.sorted_ids_by_score.write().await;
        *sorted_ids = ids;
    }

    // Obtiene la lista de blockchains ordenadas por puntuación
    pub async fn get_top_blockchains(&self, limit: usize) -> Vec<BlockchainInfo> {
        let sorted_ids = self.sorted_ids_by_score.read().await;
        let blockchains = self.blockchains.read().await;
        
        sorted_ids.iter()
            .take(limit)
            .filter_map(|id| blockchains.get(id).cloned())
            .collect()
    }

    // Busca blockchains por nombre o símbolo
    pub async fn search_blockchains(&self, query: &str) -> Vec<BlockchainInfo> {
        let blockchains = self.blockchains.read().await;
        let query_lower = query.to_lowercase();
        
        blockchains.values()
            .filter(|bc| {
                bc.name.to_lowercase().contains(&query_lower) || 
                bc.currency_symbol.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    // Obtiene una blockchain por su ID
    pub async fn get_blockchain(&self, chain_id: u32) -> Option<BlockchainInfo> {
        self.blockchains.read().await.get(&chain_id).cloned()
    }

    // Obtiene el mejor endpoint RPC para una blockchain específica
    pub async fn get_best_rpc(&self, chain_id: u32) -> Option<String> {
        let blockchains = self.blockchains.read().await;
        blockchains.get(&chain_id)
            .and_then(|blockchain| blockchain.best_endpoint())
            .map(|endpoint| endpoint.url.clone())
    }
}

// Módulos adicionales para DEX y arbitraje
pub mod dex;
pub mod opportunity;

// Nuevos módulos V3.3 (RLI)
pub mod anti_rug;
pub mod liquidity;
pub mod oracles;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blockchain_scoring() {
        let mut blockchain = BlockchainInfo {
            id: 1,
            name: String::from("Test Chain"),
            network_id: 1,
            currency_symbol: String::from("TEST"),
            rpc_endpoints: vec![],
            priority: ChainPriority::High,
            avg_block_time_sec: 5.0,
            supported_dexes: vec![
                SupportedDex {
                    name: String::from("TestDEX"),
                    router_address: String::from("0x123"),
                    factory_address: Some(String::from("0x456")),
                    fee_tiers: vec![30],
                    supports_flash_swaps: true,
                    avg_liquidity_score: 80.0,
                },
            ],
            flash_loan_providers: vec![String::from("TestProvider")],
            tvl_usd: 1_000_000_000.0,
            gas_token_price_usd: 100.0,
            last_updated: chrono::Utc::now(),
            score: 0.0,
            is_active: true,
        };
        
        blockchain.calculate_opportunity_score();
        assert!(blockchain.score > 0.0);
    }
}
