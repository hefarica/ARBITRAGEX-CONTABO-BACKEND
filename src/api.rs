// ArbitrageX Supreme V3.3 (RLI) - API para Blockchain Selector
// Implementación de rutas API para acceso a datos del selector de blockchains, DEXs y oportunidades
// REGLAS ABSOLUTAS: DATOS REALES ÚNICAMENTE - Verificación exhaustiva de todas las respuestas

use crate::blockchain_selector::{BlockchainInfo, BlockchainSelector, ChainPriority};
use crate::blockchain_selector::dex::{DexInfo, DexSelector, TradingPair};
use crate::blockchain_selector::opportunity::{ArbitrageOpportunity, OpportunityDetector, DetectionConfig, OpportunityStatus, RiskLevel};
use crate::blockchain_selector::anti_rug::{AntiRugSystem, TokenSecurity};
use crate::blockchain_selector::liquidity::{LiquidityAnalyzer, LiquidityDepthAnalysis};
use crate::blockchain_selector::oracles::{OracleManager, VerifiedPrice};
use std::sync::Arc;
use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use actix_web::http::StatusCode;
use actix_web::middleware::{Logger, NormalizePath};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use futures::{StreamExt, Stream};
use tokio_stream::wrappers::UnboundedReceiverStream;
use std::time::Duration;
use std::collections::HashMap;
use log::{info, error, warn};

// Estructuras para las solicitudes
#[derive(Debug, Deserialize)]
pub struct ChainIdPath {
    chain_id: u32,
}

#[derive(Debug, Deserialize)]
pub struct OpportunityIdPath {
    id: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryLimit {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DexQueryPath {
    chain_id: u32,
    dex_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    query: String,
}

// Estructuras para las respuestas
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    message: Option<String>,
    data: Option<T>,
    timestamp: i64,
}

#[derive(Debug, Serialize)]
pub struct BlockchainListResponse {
    blockchains: Vec<BlockchainInfo>,
    total: usize,
    filtered: bool,
}

#[derive(Debug, Serialize)]
pub struct DexListResponse {
    dexes: Vec<DexInfo>,
    total: usize,
    chain_id: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct TradingPairListResponse {
    pairs: Vec<TradingPair>,
    total: usize,
    dex_name: String,
    chain_id: u32,
}

#[derive(Debug, Serialize)]
pub struct OpportunityListResponse {
    opportunities: Vec<ArbitrageOpportunity>,
    total: usize,
    chain_id: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct SystemStatusResponse {
    active_blockchains: Vec<u32>,
    scanning_status: HashMap<u32, bool>,
    opportunity_count: usize,
    last_detected: Option<i64>,
    uptime_seconds: u64,
    version: String,
}

// AppState para compartir componentes entre rutas
pub struct AppState {
    blockchain_selector: Arc<BlockchainSelector>,
    dex_selector: Arc<DexSelector>,
    opportunity_detector: Arc<OpportunityDetector>,
    anti_rug_system: Arc<AntiRugSystem>,
    liquidity_analyzer: Arc<LiquidityAnalyzer>,
    oracle_manager: Arc<OracleManager>,
    start_time: std::time::Instant,
}

// Inicializador del módulo API
pub async fn init_api(
    blockchain_selector: Arc<BlockchainSelector>,
    dex_selector: Arc<DexSelector>,
    opportunity_detector: Arc<OpportunityDetector>,
    anti_rug_system: Arc<AntiRugSystem>,
    liquidity_analyzer: Arc<LiquidityAnalyzer>,
    oracle_manager: Arc<OracleManager>,
    bind_address: &str,
) -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        blockchain_selector,
        dex_selector,
        opportunity_detector,
        anti_rug_system,
        liquidity_analyzer,
        oracle_manager,
        start_time: std::time::Instant::now(),
    });

    info!("Iniciando servidor API en {}", bind_address);

    HttpServer::new(move || {
        // Configuración CORS para permitir solicitudes desde el frontend
        let cors = Cors::default()
            .allowed_origin("https://lovable-frontend.pages.dev")  // Frontend principal
            .allowed_origin("http://localhost:3000")               // Desarrollo local
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .wrap(NormalizePath::default())
            .wrap(cors)
            // Rutas de blockchains
            .service(
                web::scope("/api/blockchains")
                    .route("", web::get().to(get_all_blockchains))
                    .route("/top", web::get().to(get_top_blockchains))
                    .route("/search", web::get().to(search_blockchains))
                    .route("/{chain_id}", web::get().to(get_blockchain_by_id))
            )
            // Rutas de DEXs
            .service(
                web::scope("/api/dexes")
                    .route("", web::get().to(get_all_dexes))
                    .route("/search", web::get().to(search_dexes))
                    .route("/chain/{chain_id}", web::get().to(get_dexes_by_chain))
                    .route("/{chain_id}/{dex_name}", web::get().to(get_dex_info))
            )
            // Rutas de pares de trading
            .service(
                web::scope("/api/pairs")
                    .route("", web::get().to(get_top_trading_pairs))
                    .route("/chain/{chain_id}", web::get().to(get_pairs_by_chain))
                    .route("/dex/{chain_id}/{dex_name}", web::get().to(get_pairs_by_dex))
            )
            // Rutas de oportunidades
            .service(
                web::scope("/api/opportunities")
                    .route("", web::get().to(get_all_opportunities))
                    .route("/chain/{chain_id}", web::get().to(get_opportunities_by_chain))
                    .route("/{id}", web::get().to(get_opportunity_by_id))
            )
            // Rutas de sistema
            .service(
                web::scope("/api/system")
                    .route("/status", web::get().to(get_system_status))
                    .route("/scan/start/{chain_id}", web::post().to(start_chain_scan))
                    .route("/scan/stop/{chain_id}", web::post().to(stop_chain_scan))
            )
            // Nuevas rutas V3.3 (RLI) - Anti-Rug System
            .service(
                web::scope("/api/security")
                    .route("/token/{chain_id}/{token_address}", web::get().to(get_token_security))
                    .route("/blacklist", web::get().to(get_blacklisted_tokens))
                    .route("/pair/{chain_id}/{pair_id}", web::get().to(evaluate_pair_security))
            )
            // Nuevas rutas V3.3 (RLI) - Liquidity Analyzer
            .service(
                web::scope("/api/liquidity")
                    .route("/depth/{chain_id}/{pair_id}", web::get().to(get_liquidity_depth))
                    .route("/concentration/{chain_id}/{pair_id}", web::get().to(get_liquidity_concentration))
                    .route("/top-pairs", web::get().to(get_top_liquidity_pairs))
            )
            // Nuevas rutas V3.3 (RLI) - Oracle Manager
            .service(
                web::scope("/api/oracle")
                    .route("/price/{chain_id}/{base_token}/{quote_token}", web::get().to(get_token_price))
                    .route("/confidence/{chain_id}/{token_a}/{token_b}", web::get().to(get_price_confidence))
            )
            // WebSockets para streaming de oportunidades
            .route("/ws/opportunities", web::get().to(opportunity_websocket))
    })
    .bind(bind_address)?
    .run()
    .await
}

// Handlers para rutas de blockchain
async fn get_all_blockchains(
    state: web::Data<AppState>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(100);
    
    // Obtener todas las blockchains disponibles
    let blockchains = match state.blockchain_selector.get_top_blockchains(limit).await {
        Ok(bcs) => bcs,
        Err(_) => Vec::new(),
    };
    
    let response = BlockchainListResponse {
        total: blockchains.len(),
        filtered: false,
        blockchains,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_top_blockchains(
    state: web::Data<AppState>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(10);
    
    // Obtener blockchains ordenadas por puntuación
    let blockchains = match state.blockchain_selector.get_top_blockchains(limit).await {
        Ok(bcs) => bcs,
        Err(_) => Vec::new(),
    };
    
    let response = BlockchainListResponse {
        total: blockchains.len(),
        filtered: true,
        blockchains,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn search_blockchains(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    // Buscar blockchains por nombre o símbolo
    let blockchains = match state.blockchain_selector.search_blockchains(&query.query).await {
        Ok(bcs) => bcs,
        Err(_) => Vec::new(),
    };
    
    let response = BlockchainListResponse {
        total: blockchains.len(),
        filtered: true,
        blockchains,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_blockchain_by_id(
    state: web::Data<AppState>,
    path: web::Path<ChainIdPath>,
) -> impl Responder {
    // Obtener una blockchain específica por ID
    let blockchain = match state.blockchain_selector.get_blockchain(path.chain_id).await {
        Ok(Some(bc)) => bc,
        Ok(None) => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Blockchain con ID {} no encontrada", path.chain_id)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        },
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: Some("Error interno al obtener blockchain".to_string()),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(blockchain),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

// Handlers para rutas de DEXs
async fn get_all_dexes(
    state: web::Data<AppState>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(100);
    
    // Obtener todos los DEXs disponibles
    // Nota: Para simplicidad, unimos los DEXs de las principales blockchains
    let mut all_dexes = Vec::new();
    
    // Obtener DEXs de las blockchains principales
    let top_chains = match state.blockchain_selector.get_top_blockchains(5).await {
        Ok(chains) => chains,
        Err(_) => Vec::new(),
    };
    
    for chain in top_chains {
        let chain_dexes = state.dex_selector.get_dexes_by_chain(chain.id).await;
        all_dexes.extend(chain_dexes);
        
        if all_dexes.len() >= limit {
            all_dexes.truncate(limit);
            break;
        }
    }
    
    let response = DexListResponse {
        total: all_dexes.len(),
        chain_id: None,
        dexes: all_dexes,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn search_dexes(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    // Buscar DEXs por nombre
    let dexes = state.dex_selector.search_dexes(&query.query).await;
    
    let response = DexListResponse {
        total: dexes.len(),
        chain_id: None,
        dexes,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_dexes_by_chain(
    state: web::Data<AppState>,
    path: web::Path<ChainIdPath>,
) -> impl Responder {
    // Obtener DEXs de una blockchain específica
    let dexes = state.dex_selector.get_dexes_by_chain(path.chain_id).await;
    
    let response = DexListResponse {
        total: dexes.len(),
        chain_id: Some(path.chain_id),
        dexes,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_dex_info(
    state: web::Data<AppState>,
    path: web::Path<DexQueryPath>,
) -> impl Responder {
    // Obtener información de un DEX específico
    let dex = match state.dex_selector.get_dex_info(path.chain_id, &path.dex_name).await {
        Some(dex) => dex,
        None => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("DEX {} no encontrado en blockchain {}", path.dex_name, path.chain_id)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(dex),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

// Handlers para rutas de pares de trading
async fn get_top_trading_pairs(
    state: web::Data<AppState>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(20);
    
    // Obtener los pares de trading con mayor volumen
    let pairs = state.dex_selector.get_top_pairs_by_volume(limit).await;
    
    let response = TradingPairListResponse {
        total: pairs.len(),
        dex_name: String::from("*"),
        chain_id: 0,
        pairs,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_pairs_by_chain(
    state: web::Data<AppState>,
    path: web::Path<ChainIdPath>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50);
    let chain_id = path.chain_id;
    
    // Obtener DEXs para esta blockchain
    let dexes = state.dex_selector.get_dexes_by_chain(chain_id).await;
    let mut all_pairs = Vec::new();
    
    // Para cada DEX, obtener sus pares
    for dex in dexes {
        let pairs = state.dex_selector.get_trading_pairs(chain_id, &dex.name).await;
        all_pairs.extend(pairs);
        
        if all_pairs.len() >= limit {
            all_pairs.truncate(limit);
            break;
        }
    }
    
    // Ordenar por volumen
    all_pairs.sort_by(|a, b| {
        b.volume_24h_usd.partial_cmp(&a.volume_24h_usd).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let response = TradingPairListResponse {
        total: all_pairs.len(),
        dex_name: String::from("*"),
        chain_id,
        pairs: all_pairs,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_pairs_by_dex(
    state: web::Data<AppState>,
    path: web::Path<DexQueryPath>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50);
    
    // Obtener pares de trading para un DEX específico
    let mut pairs = state.dex_selector.get_trading_pairs(path.chain_id, &path.dex_name).await;
    
    // Ordenar por volumen y limitar
    pairs.sort_by(|a, b| {
        b.volume_24h_usd.partial_cmp(&a.volume_24h_usd).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    if pairs.len() > limit {
        pairs.truncate(limit);
    }
    
    let response = TradingPairListResponse {
        total: pairs.len(),
        dex_name: path.dex_name.clone(),
        chain_id: path.chain_id,
        pairs,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

// Handlers para rutas de oportunidades
async fn get_all_opportunities(
    state: web::Data<AppState>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50);
    let mut all_opportunities = Vec::new();
    
    // Obtener las blockchains activas
    let top_chains = match state.blockchain_selector.get_top_blockchains(10).await {
        Ok(chains) => chains,
        Err(_) => Vec::new(),
    };
    
    // Para cada blockchain, obtener oportunidades
    for chain in top_chains {
        let chain_opps = state.opportunity_detector.get_opportunities_for_chain(chain.id, limit / 2).await;
        all_opportunities.extend(chain_opps);
        
        if all_opportunities.len() >= limit {
            all_opportunities.truncate(limit);
            break;
        }
    }
    
    // Ordenar por beneficio neto (de mayor a menor)
    all_opportunities.sort_by(|a, b| {
        b.net_profit_usd.partial_cmp(&a.net_profit_usd).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let response = OpportunityListResponse {
        total: all_opportunities.len(),
        chain_id: None,
        opportunities: all_opportunities,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_opportunities_by_chain(
    state: web::Data<AppState>,
    path: web::Path<ChainIdPath>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(20);
    
    // Obtener oportunidades para una blockchain específica
    let opportunities = state.opportunity_detector.get_opportunities_for_chain(path.chain_id, limit).await;
    
    let response = OpportunityListResponse {
        total: opportunities.len(),
        chain_id: Some(path.chain_id),
        opportunities,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn get_opportunity_by_id(
    state: web::Data<AppState>,
    path: web::Path<OpportunityIdPath>,
) -> impl Responder {
    // Obtener una oportunidad específica por ID
    let opportunity = match state.opportunity_detector.get_opportunity(&path.id).await {
        Some(opp) => opp,
        None => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Oportunidad con ID {} no encontrada", path.id)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(opportunity),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

// Handlers para rutas de sistema
async fn get_system_status(state: web::Data<AppState>) -> impl Responder {
    // Crear mapa de estado de escaneo
    let mut scanning_status = HashMap::new();
    let active_scans = state.opportunity_detector.get_active_scans().await;
    
    // Obtener blockchains principales
    let top_chains = match state.blockchain_selector.get_top_blockchains(20).await {
        Ok(chains) => chains,
        Err(_) => Vec::new(),
    };
    
    // Crear mapa de estado
    let active_chain_ids: Vec<u32> = top_chains.iter().map(|bc| bc.id).collect();
    
    for &chain_id in &active_chain_ids {
        scanning_status.insert(chain_id, active_scans.contains(&chain_id));
    }
    
    // Obtener contador de oportunidades y última detección
    let (opportunity_count, last_detected) = state.opportunity_detector.get_stats().await;
    
    // Crear respuesta
    let response = SystemStatusResponse {
        active_blockchains: active_chain_ids,
        scanning_status,
        opportunity_count,
        last_detected: last_detected.map(|dt| dt.timestamp()),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn start_chain_scan(
    state: web::Data<AppState>,
    path: web::Path<ChainIdPath>,
) -> impl Responder {
    // Iniciar escaneo para una blockchain específica
    match state.opportunity_detector.start_scanning_chain(path.chain_id).await {
        Ok(_) => {
            HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                message: Some(format!("Escaneo iniciado para blockchain {}", path.chain_id)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Error iniciando escaneo: {}", e)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            })
        }
    }
}

async fn stop_chain_scan(
    state: web::Data<AppState>,
    path: web::Path<ChainIdPath>,
) -> impl Responder {
    // Detener escaneo para una blockchain específica
    state.opportunity_detector.stop_scanning_chain(path.chain_id).await;
    
    HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: Some(format!("Escaneo detenido para blockchain {}", path.chain_id)),
        data: None,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

// WebSocket para oportunidades en tiempo real
async fn opportunity_websocket(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    // Crear WebSocket
    let (response, session, msg_stream) = actix_web::web::block(move || {
        actix_web_actors::ws::WebsocketContext::start(
            OpportunityWebSocket::new(state.opportunity_detector.clone()),
            &req,
            stream,
        )
    })
    .await??;
    
    Ok(response)
}

// Actor WebSocket para streaming de oportunidades
struct OpportunityWebSocket {
    detector: Arc<OpportunityDetector>,
    rx: Option<mpsc::Receiver<ArbitrageOpportunity>>,
}

impl OpportunityWebSocket {
    fn new(detector: Arc<OpportunityDetector>) -> Self {
        Self {
            detector,
            rx: None,
        }
    }
}

impl actix::Actor for OpportunityWebSocket {
    type Context = actix_web_actors::ws::WebsocketContext<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        // Suscribirse al detector de oportunidades
        let rx = self.detector.subscribe();
        self.rx = Some(rx);
        
        // Configurar recepción de oportunidades
        let rx = self.rx.take().unwrap();
        let stream = UnboundedReceiverStream::new(rx);
        
        ctx.add_stream(stream);
    }
}

impl actix::StreamHandler<Result<ArbitrageOpportunity, std::io::Error>> for OpportunityWebSocket {
    fn handle(&mut self, item: Result<ArbitrageOpportunity, std::io::Error>, ctx: &mut Self::Context) {
        match item {
            Ok(opportunity) => {
                // Enviar oportunidad como JSON
                if let Ok(json) = serde_json::to_string(&opportunity) {
                    ctx.text(json);
                }
            },
            Err(e) => {
                error!("Error en streaming de oportunidades: {}", e);
                ctx.close(None);
            }
        }
    }
}

impl actix_web_actors::ws::StreamHandler<Result<actix_web::web::BytesMut, actix_web::web::Error>> for OpportunityWebSocket {
    fn handle(&mut self, msg: Result<actix_web::web::BytesMut, actix_web::web::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(_) => {
                // Ping/pong
                ctx.pong(&[]);
            },
            Err(e) => {
                error!("Error en WebSocket: {}", e);
                ctx.close(None);
            }
        }
    }
}

// =================== NUEVOS HANDLERS V3.3 (RLI) ===================

// Estructuras para las peticiones de rutas de seguridad
#[derive(Debug, Deserialize)]
pub struct TokenSecurityPath {
    chain_id: u32,
    token_address: String,
}

#[derive(Debug, Deserialize)]
pub struct PairSecurityPath {
    chain_id: u32,
    pair_id: String,
}

// Estructuras para las respuestas de seguridad
#[derive(Debug, Serialize)]
pub struct TokenSecurityResponse {
    token_address: String,
    symbol: String,
    security_score: u32,
    warning_flags: Vec<String>,
    is_blacklisted: bool,
    is_whitelisted: bool,
    security_details: Option<TokenSecurity>,
}

#[derive(Debug, Serialize)]
pub struct BlacklistResponse {
    blacklisted_tokens: Vec<String>,
    total: usize,
    last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct PairSecurityResponse {
    pair_id: String,
    security_score: u32,
    token0_security: TokenSecurityResponse,
    token1_security: TokenSecurityResponse,
    recommendation: String,
}

// Estructuras para las respuestas de liquidez
#[derive(Debug, Serialize)]
pub struct LiquidityPairResponse {
    pair: TradingPair,
    depth_analysis: LiquidityDepthAnalysis,
    optimal_trade_size: f64,
}

#[derive(Debug, Serialize)]
pub struct TopLiquidityPairsResponse {
    pairs: Vec<LiquidityPairResponse>,
    total: usize,
}

// Estructuras para las respuestas de oráculos
#[derive(Debug, Serialize)]
pub struct OraclePriceResponse {
    base_token: String,
    quote_token: String,
    price: f64,
    confidence: f64,
    sources_count: usize,
    is_fresh: bool,
    last_verified: i64,
}

#[derive(Debug, Deserialize)]
pub struct OraclePricePath {
    chain_id: u32,
    base_token: String,
    quote_token: String,
}

#[derive(Debug, Deserialize)]
pub struct OracleConfidencePath {
    chain_id: u32,
    token_a: String,
    token_b: String,
}

#[derive(Debug, Serialize)]
pub struct PriceConfidenceResponse {
    token_a: String,
    token_b: String,
    dex_price_ratio: f64,
    oracle_price: f64,
    confidence_score: u32,
    deviation_percent: f64,
}

// Handlers para Anti-Rug System
async fn get_token_security(
    state: web::Data<AppState>,
    path: web::Path<TokenSecurityPath>,
) -> impl Responder {
    // Analizar seguridad del token
    match state.anti_rug_system.analyze_token_security(path.chain_id, &path.token_address).await {
        Ok(security) => {
            let is_blacklisted = state.anti_rug_system.is_blacklisted(path.chain_id, &path.token_address).await;
            let is_whitelisted = state.anti_rug_system.is_whitelisted(path.chain_id, &path.token_address).await;
            
            let response = TokenSecurityResponse {
                token_address: path.token_address.clone(),
                symbol: security.symbol.clone(),
                security_score: security.security_score,
                warning_flags: security.warning_flags.clone(),
                is_blacklisted,
                is_whitelisted,
                security_details: Some(security),
            };
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: None,
                data: Some(response),
                timestamp: chrono::Utc::now().timestamp(),
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Error analizando seguridad del token: {}", e)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            })
        }
    }
}

async fn get_blacklisted_tokens(state: web::Data<AppState>) -> impl Responder {
    // TODO: Implementar obtención real de tokens blacklisteados
    // Por ahora devolvemos una respuesta vacía
    let response = BlacklistResponse {
        blacklisted_tokens: Vec::new(),
        total: 0,
        last_updated: chrono::Utc::now().timestamp(),
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

async fn evaluate_pair_security(
    state: web::Data<AppState>,
    path: web::Path<PairSecurityPath>,
) -> impl Responder {
    // Primero obtenemos información del par
    let pair_id = path.pair_id.clone();
    let chain_id = path.chain_id;
    
    // Buscar el par en todos los DEXs de esta blockchain
    let mut target_pair: Option<TradingPair> = None;
    
    let dexes = state.dex_selector.get_dexes_by_chain(chain_id).await;
    for dex in dexes {
        let pairs = state.dex_selector.get_trading_pairs(chain_id, &dex.name).await;
        for pair in pairs {
            if pair.id == pair_id {
                target_pair = Some(pair);
                break;
            }
        }
        if target_pair.is_some() {
            break;
        }
    }
    
    // Si no encontramos el par, error
    let pair = match target_pair {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Par {} no encontrado en blockchain {}", pair_id, chain_id)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    };
    
    // Evaluar seguridad del par
    let security_score = match state.anti_rug_system.evaluate_pair_security(&pair).await {
        Ok(score) => score,
        Err(_) => 0, // Si hay error, considerar riesgo máximo
    };
    
    // Obtener seguridad de cada token
    let token0_security = match state.anti_rug_system.analyze_token_security(chain_id, &pair.token0_address).await {
        Ok(security) => {
            let is_blacklisted = state.anti_rug_system.is_blacklisted(chain_id, &pair.token0_address).await;
            let is_whitelisted = state.anti_rug_system.is_whitelisted(chain_id, &pair.token0_address).await;
            
            TokenSecurityResponse {
                token_address: pair.token0_address.clone(),
                symbol: pair.token0_symbol.clone(),
                security_score: security.security_score,
                warning_flags: security.warning_flags.clone(),
                is_blacklisted,
                is_whitelisted,
                security_details: None, // Omitimos detalles completos para reducir tamaño
            }
        },
        Err(_) => {
            TokenSecurityResponse {
                token_address: pair.token0_address.clone(),
                symbol: pair.token0_symbol.clone(),
                security_score: 0,
                warning_flags: vec!["No se pudo analizar seguridad".to_string()],
                is_blacklisted: false,
                is_whitelisted: false,
                security_details: None,
            }
        }
    };
    
    let token1_security = match state.anti_rug_system.analyze_token_security(chain_id, &pair.token1_address).await {
        Ok(security) => {
            let is_blacklisted = state.anti_rug_system.is_blacklisted(chain_id, &pair.token1_address).await;
            let is_whitelisted = state.anti_rug_system.is_whitelisted(chain_id, &pair.token1_address).await;
            
            TokenSecurityResponse {
                token_address: pair.token1_address.clone(),
                symbol: pair.token1_symbol.clone(),
                security_score: security.security_score,
                warning_flags: security.warning_flags.clone(),
                is_blacklisted,
                is_whitelisted,
                security_details: None,
            }
        },
        Err(_) => {
            TokenSecurityResponse {
                token_address: pair.token1_address.clone(),
                symbol: pair.token1_symbol.clone(),
                security_score: 0,
                warning_flags: vec!["No se pudo analizar seguridad".to_string()],
                is_blacklisted: false,
                is_whitelisted: false,
                security_details: None,
            }
        }
    };
    
    // Determinar recomendación
    let recommendation = if security_score >= 80 {
        "SEGURO - Par verificado con alta puntuación de seguridad".to_string()
    } else if security_score >= 50 {
        "PRECAUCIÓN - Par con nivel de seguridad medio, verificar tokens".to_string()
    } else {
        "ALTO RIESGO - No recomendado para operaciones de arbitraje".to_string()
    };
    
    let response = PairSecurityResponse {
        pair_id: pair_id.clone(),
        security_score,
        token0_security,
        token1_security,
        recommendation,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

// Handlers para Liquidity Analyzer
async fn get_liquidity_depth(
    state: web::Data<AppState>,
    path: web::Path<PairSecurityPath>,
) -> impl Responder {
    let pair_id = path.pair_id.clone();
    let chain_id = path.chain_id;
    
    // Buscar el par
    let mut target_pair: Option<TradingPair> = None;
    
    let dexes = state.dex_selector.get_dexes_by_chain(chain_id).await;
    for dex in dexes {
        let pairs = state.dex_selector.get_trading_pairs(chain_id, &dex.name).await;
        for pair in pairs {
            if pair.id == pair_id {
                target_pair = Some(pair);
                break;
            }
        }
        if target_pair.is_some() {
            break;
        }
    }
    
    // Si no encontramos el par, error
    let pair = match target_pair {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Par {} no encontrado en blockchain {}", pair_id, chain_id)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    };
    
    // Analizar profundidad de liquidez
    match state.liquidity_analyzer.analyze_pair_depth(&pair).await {
        Ok(analysis) => {
            // Calcular tamaño óptimo de trade basado en el análisis
            let optimal_trade_size = analysis.max_single_trade_usd * 0.7; // 70% del máximo para minimizar slippage
            
            let response = LiquidityPairResponse {
                pair: pair.clone(),
                depth_analysis: analysis,
                optimal_trade_size,
            };
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: None,
                data: Some(response),
                timestamp: chrono::Utc::now().timestamp(),
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Error analizando profundidad de liquidez: {}", e)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            })
        }
    }
}

async fn get_liquidity_concentration(
    state: web::Data<AppState>,
    path: web::Path<PairSecurityPath>,
) -> impl Responder {
    // Esta función sería similar a get_liquidity_depth pero llamando a analyze_liquidity_concentration
    // Para simplificar, reusamos la misma lógica de búsqueda de par y llamamos al endpoint de concentración
    
    let pair_id = path.pair_id.clone();
    let chain_id = path.chain_id;
    
    // Buscar el par
    let mut target_pair: Option<TradingPair> = None;
    
    let dexes = state.dex_selector.get_dexes_by_chain(chain_id).await;
    for dex in dexes {
        let pairs = state.dex_selector.get_trading_pairs(chain_id, &dex.name).await;
        for pair in pairs {
            if pair.id == pair_id {
                target_pair = Some(pair);
                break;
            }
        }
        if target_pair.is_some() {
            break;
        }
    }
    
    // Si no encontramos el par, error
    let pair = match target_pair {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Par {} no encontrado en blockchain {}", pair_id, chain_id)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    };
    
    // Analizar concentración de liquidez
    match state.liquidity_analyzer.analyze_liquidity_concentration(&pair).await {
        Ok(concentration) => {
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: None,
                data: Some(concentration),
                timestamp: chrono::Utc::now().timestamp(),
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Error analizando concentración de liquidez: {}", e)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            })
        }
    }
}

async fn get_top_liquidity_pairs(
    state: web::Data<AppState>,
    query: web::Query<QueryLimit>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(10);
    
    // Obtener los pares de trading con mayor volumen
    let pairs = state.dex_selector.get_top_pairs_by_volume(limit * 2).await;
    
    // Analizar liquidez para cada par y filtrar los mejores
    let mut analyzed_pairs = Vec::new();
    let trade_size_usd = 10000.0; // Tamaño de referencia para evaluar
    
    // Analizamos hasta el doble de pares para luego filtrar por liquidez
    for pair in pairs {
        match state.liquidity_analyzer.analyze_pair_depth(&pair).await {
            Ok(analysis) => {
                analyzed_pairs.push(LiquidityPairResponse {
                    pair: pair.clone(),
                    depth_analysis: analysis.clone(),
                    optimal_trade_size: analysis.max_single_trade_usd * 0.7,
                });
            },
            Err(_) => continue, // Ignoramos pares con error de análisis
        }
    }
    
    // Ordenar por health_score de liquidez
    analyzed_pairs.sort_by(|a, b| {
        b.depth_analysis.overall_health_score.cmp(&a.depth_analysis.overall_health_score)
    });
    
    // Limitar al número solicitado
    if analyzed_pairs.len() > limit {
        analyzed_pairs.truncate(limit);
    }
    
    let response = TopLiquidityPairsResponse {
        pairs: analyzed_pairs,
        total: analyzed_pairs.len(),
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

// Handlers para Oracle Manager
async fn get_token_price(
    state: web::Data<AppState>,
    path: web::Path<OraclePricePath>,
) -> impl Responder {
    // Obtener precio verificado del oráculo
    match state.oracle_manager.get_verified_price(
        path.chain_id, 
        &path.base_token, 
        &path.quote_token
    ).await {
        Ok(price) => {
            let response = OraclePriceResponse {
                base_token: price.base_token,
                quote_token: price.quote_token,
                price: price.price,
                confidence: price.confidence,
                sources_count: price.sources.len(),
                is_fresh: price.is_fresh,
                last_verified: price.last_verified.timestamp(),
            };
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: None,
                data: Some(response),
                timestamp: chrono::Utc::now().timestamp(),
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Error obteniendo precio del oráculo: {}", e)),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            })
        }
    }
}

async fn get_price_confidence(
    state: web::Data<AppState>,
    path: web::Path<OracleConfidencePath>,
    web::Query(query): web::Query<HashMap<String, String>>,
) -> impl Responder {
    // Obtener parámetro dex_price_ratio de la query
    let dex_price_ratio = match query.get("dex_price_ratio").map(|s| s.parse::<f64>()) {
        Some(Ok(ratio)) => ratio,
        _ => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                message: Some("Parámetro dex_price_ratio requerido y debe ser un número".to_string()),
                data: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
        }
    };
    
    // Calcular puntuación de confianza
    let confidence_score = state.oracle_manager.calculate_price_confidence(
        path.chain_id,
        &path.token_a,
        &path.token_b,
        dex_price_ratio,
    ).await;
    
    // Obtener precio de oráculo para calcular desviación
    let oracle_price = match state.oracle_manager.get_verified_price(
        path.chain_id,
        &path.token_a,
        &path.token_b,
    ).await {
        Ok(price) => price.price,
        Err(_) => 0.0, // Si no hay precio de oráculo, usamos 0
    };
    
    // Calcular desviación porcentual
    let deviation_percent = if oracle_price > 0.0 {
        ((dex_price_ratio - oracle_price).abs() / oracle_price) * 100.0
    } else {
        0.0
    };
    
    let response = PriceConfidenceResponse {
        token_a: path.token_a.clone(),
        token_b: path.token_b.clone(),
        dex_price_ratio,
        oracle_price,
        confidence_score,
        deviation_percent,
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(response),
        timestamp: chrono::Utc::now().timestamp(),
    })
}
