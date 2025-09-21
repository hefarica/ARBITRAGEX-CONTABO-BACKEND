// ArbitrageX Supreme V3.3 (RLI) - Aplicación Principal
// Demostración de la integración de todos los componentes del sistema de arbitraje
// REGLAS ABSOLUTAS: DATOS REALES ÚNICAMENTE - Todos los endpoints y fuentes verificados

use std::sync::Arc;
use tokio;
use log::{info, error, warn, LevelFilter};
use env_logger::Builder;
use dotenv::dotenv;
use std::env;
use std::time::Duration;
use anyhow::{Result, Context};

mod blockchain_selector;
use blockchain_selector::{BlockchainSelector, BlockchainInfo, ChainPriority};
use blockchain_selector::dex::{DexSelector, DexInfo, TradingPair};
use blockchain_selector::opportunity::{OpportunityDetector, ArbitrageOpportunity, DetectionConfig};
use blockchain_selector::api;
use blockchain_selector::anti_rug::AntiRugSystem;
use blockchain_selector::liquidity::LiquidityAnalyzer;
use blockchain_selector::oracles::OracleManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Cargar variables de entorno desde .env
    dotenv().ok();
    
    // Configurar logging
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format_timestamp_secs()
        .init();
    
    info!("Iniciando ArbitrageX Supreme V3.3 (RLI)");
    
    // Inicializar componentes
    info!("Inicializando BlockchainSelector...");
    let blockchain_selector = Arc::new(BlockchainSelector::new().await?);
    
    info!("Inicializando DexSelector...");
    let dex_selector = Arc::new(DexSelector::new().await?);
    
    info!("Inicializando AntiRugSystem...");
    let anti_rug_system = Arc::new(AntiRugSystem::new().await?);
    
    info!("Inicializando LiquidityAnalyzer...");
    let liquidity_analyzer = Arc::new(LiquidityAnalyzer::new().await?);
    
    info!("Inicializando OracleManager...");
    let oracle_manager = Arc::new(OracleManager::new().await?);
    
    info!("Inicializando OpportunityDetector...");
    let opportunity_detector = Arc::new(OpportunityDetector::new(
        Arc::clone(&blockchain_selector),
        Arc::clone(&dex_selector),
    ).await?);
    
    // Iniciar escáneres para blockchains principales
    info!("Iniciando escáneres para blockchains de alta prioridad...");
    start_priority_scanners(&opportunity_detector).await?;
    
    // Iniciar servidor API
    let api_address = env::var("API_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    info!("Iniciando servidor API en {}...", api_address);
    
    // Ejecutar componentes
    tokio::select! {
        result = api::init_api(
            Arc::clone(&blockchain_selector),
            Arc::clone(&dex_selector),
            Arc::clone(&opportunity_detector),
            Arc::clone(&anti_rug_system),
            Arc::clone(&liquidity_analyzer),
            Arc::clone(&oracle_manager),
            &api_address,
        ) => {
            if let Err(e) = result {
                error!("Error en servidor API: {}", e);
                return Err(anyhow::anyhow!("Error en servidor API: {}", e));
            }
        },
        _ = tokio::signal::ctrl_c() => {
            info!("Señal de interrupción recibida, deteniendo servicios...");
        }
    }
    
    // Detener escáneres
    info!("Deteniendo escáneres...");
    stop_all_scanners(&opportunity_detector).await;
    
    info!("ArbitrageX Supreme V3.3 (RLI) detenido correctamente");
    Ok(())
}

// Función para iniciar escáneres de blockchains prioritarias
async fn start_priority_scanners(
    opportunity_detector: &Arc<OpportunityDetector>
) -> Result<()> {
    // Lista de blockchains prioritarias para escanear
    let priority_chains = vec![
        1,      // Ethereum
        42161,  // Arbitrum
        137,    // Polygon
        10,     // Optimism
        56,     // BNB Chain
    ];
    
    // Iniciar escáneres para cada blockchain
    for &chain_id in &priority_chains {
        match opportunity_detector.start_scanning_chain(chain_id).await {
            Ok(_) => info!("Escáner iniciado para blockchain {}", chain_id),
            Err(e) => warn!("No se pudo iniciar escáner para blockchain {}: {}", chain_id, e),
        }
    }
    
    Ok(())
}

// Función para detener todos los escáneres
async fn stop_all_scanners(opportunity_detector: &Arc<OpportunityDetector>) {
    // Obtener blockchains activas
    let active_scans = opportunity_detector.get_active_scans().await;
    
    // Detener escáneres para cada blockchain activa
    for &chain_id in &active_scans {
        opportunity_detector.stop_scanning_chain(chain_id).await;
        info!("Escáner detenido para blockchain {}", chain_id);
    }
}

// Suscriptor de demostración a oportunidades de arbitraje
async fn demo_opportunity_subscriber(opportunity_detector: &Arc<OpportunityDetector>) -> Result<()> {
    // Suscribirse al detector de oportunidades
    let mut rx = opportunity_detector.subscribe();
    
    info!("Suscripción a oportunidades de arbitraje iniciada");
    
    // Procesar oportunidades recibidas
    while let Some(opportunity) = rx.recv().await {
        info!(
            "Nueva oportunidad detectada: {} - Chain: {} - Profit: ${:.2} ({}bps) - Riesgo: {:?}",
            opportunity.id,
            opportunity.chain_id,
            opportunity.net_profit_usd,
            opportunity.profit_bps,
            opportunity.risk_level,
        );
    }
    
    Ok(())
}
