use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Json, Sse},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::TcpListener,
    sync::broadcast,
    time::{interval, sleep},
};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

// Core data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: String,
    pub opportunity_type: String,
    pub chain: String,
    pub tokens: Vec<TokenInfo>,
    pub dexes: Vec<DexInfo>,
    pub profit_estimate_usd: f64,
    pub gas_estimate: u64,
    pub gas_price_gwei: u64,
    pub confidence_score: f64,
    pub risk_score: f64,
    pub transaction_data: Option<serde_json::Value>,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub amount: String,
    pub price_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexInfo {
    pub name: String,
    pub router: String,
    pub factory: String,
    pub fee_bps: u16,
    pub liquidity_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub opportunity_id: String,
    pub strategy: String,
    pub max_gas_price_gwei: Option<u64>,
    pub slippage_tolerance: Option<f64>,
    pub deadline_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub execution_id: String,
    pub status: String,
    pub estimated_profit_usd: f64,
    pub estimated_gas_cost_usd: f64,
    pub confidence_score: f64,
    pub execution_time_estimate_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
    pub active_opportunities: usize,
    pub total_executions: u64,
    pub success_rate: f64,
    pub avg_profit_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub opportunities_per_minute: f64,
    pub executions_per_minute: f64,
    pub success_rate: f64,
    pub avg_profit_usd: f64,
    pub avg_gas_cost_usd: f64,
    pub total_volume_usd: f64,
    pub active_chains: Vec<String>,
    pub top_strategies: Vec<StrategyMetric>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyMetric {
    pub name: String,
    pub count: u64,
    pub success_rate: f64,
    pub avg_profit_usd: f64,
}

#[derive(Debug, Deserialize)]
pub struct OpportunityQuery {
    pub chain: Option<String>,
    pub min_profit_usd: Option<f64>,
    pub max_risk_score: Option<f64>,
    pub strategy: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

// Application state
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub opportunities: Arc<DashMap<String, Opportunity>>,
    pub metrics: Arc<RwLock<SystemMetrics>>,
    pub config: Arc<AppConfig>,
    pub event_sender: broadcast::Sender<OpportunityEvent>,
    pub start_time: SystemTime,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub total_opportunities: u64,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub total_profit_usd: f64,
    pub total_gas_cost_usd: f64,
    pub opportunities_per_minute: f64,
    pub executions_per_minute: f64,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub port: u16,
    pub max_opportunities: usize,
    pub min_profit_threshold_usd: f64,
    pub max_risk_score: f64,
    pub opportunity_ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
pub enum OpportunityEvent {
    Created(Opportunity),
    Updated(Opportunity),
    Executed(String),
    Expired(String),
}

// API handlers
#[instrument(skip(state))]
async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    let uptime = state.start_time.elapsed().unwrap_or_default().as_secs();
    let active_opportunities = state.opportunities.len();
    
    let metrics = state.metrics.read().unwrap();
    let success_rate = if metrics.total_executions > 0 {
        metrics.successful_executions as f64 / metrics.total_executions as f64
    } else {
        0.0
    };
    
    let avg_profit = if metrics.successful_executions > 0 {
        metrics.total_profit_usd / metrics.successful_executions as f64
    } else {
        0.0
    };
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        version: "3.0.0".to_string(),
        uptime_seconds: uptime,
        active_opportunities,
        total_executions: metrics.total_executions,
        success_rate,
        avg_profit_usd: avg_profit,
    };
    
    Ok(Json(response))
}

#[instrument(skip(state))]
async fn get_opportunities(
    State(state): State<AppState>,
    Query(params): Query<OpportunityQuery>,
) -> Result<Json<Vec<Opportunity>>, StatusCode> {
    let mut opportunities: Vec<Opportunity> = state
        .opportunities
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    
    // Apply filters
    if let Some(chain) = &params.chain {
        opportunities.retain(|op| op.chain == *chain);
    }
    
    if let Some(min_profit) = params.min_profit_usd {
        opportunities.retain(|op| op.profit_estimate_usd >= min_profit);
    }
    
    if let Some(max_risk) = params.max_risk_score {
        opportunities.retain(|op| op.risk_score <= max_risk);
    }
    
    if let Some(strategy) = &params.strategy {
        opportunities.retain(|op| op.opportunity_type == *strategy);
    }
    
    // Sort by profit descending
    opportunities.sort_by(|a, b| {
        b.profit_estimate_usd.partial_cmp(&a.profit_estimate_usd).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Apply pagination
    let offset = params.offset.unwrap_or(0) as usize;
    let limit = params.limit.unwrap_or(100) as usize;
    
    let paginated: Vec<Opportunity> = opportunities
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    Ok(Json(paginated))
}

#[instrument(skip(state))]
async fn get_opportunity(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Opportunity>, StatusCode> {
    match state.opportunities.get(&id) {
        Some(opportunity) => Ok(Json(opportunity.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[instrument(skip(state))]
async fn execute_opportunity(
    State(state): State<AppState>,
    Json(request): Json<ExecutionRequest>,
) -> Result<Json<ExecutionResponse>, StatusCode> {
    let opportunity = match state.opportunities.get(&request.opportunity_id) {
        Some(op) => op.clone(),
        None => return Err(StatusCode::NOT_FOUND),
    };
    
    // Validate opportunity is still valid
    if opportunity.expires_at < Utc::now() {
        return Err(StatusCode::GONE);
    }
    
    if opportunity.status != "detected" {
        return Err(StatusCode::CONFLICT);
    }
    
    // Generate execution ID
    let execution_id = Uuid::new_v4().to_string();
    
    // Calculate execution parameters
    let gas_price_gwei = request.max_gas_price_gwei.unwrap_or(opportunity.gas_price_gwei);
    let estimated_gas_cost_usd = (opportunity.gas_estimate as f64 * gas_price_gwei as f64 * 1e-9) * 2000.0; // Assume $2000 ETH
    let net_profit = opportunity.profit_estimate_usd - estimated_gas_cost_usd;
    
    if net_profit <= 0.0 {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    
    // Update opportunity status
    let mut updated_opportunity = opportunity.clone();
    updated_opportunity.status = "executing".to_string();
    updated_opportunity.updated_at = Utc::now();
    state.opportunities.insert(request.opportunity_id.clone(), updated_opportunity.clone());
    
    // Send execution event
    let _ = state.event_sender.send(OpportunityEvent::Executed(request.opportunity_id));
    
    // Store execution in database
    if let Err(e) = store_execution(&state.db, &execution_id, &opportunity, &request).await {
        error!("Failed to store execution: {}", e);
    }
    
    let response = ExecutionResponse {
        execution_id,
        status: "submitted".to_string(),
        estimated_profit_usd: net_profit,
        estimated_gas_cost_usd,
        confidence_score: opportunity.confidence_score,
        execution_time_estimate_ms: 5000, // 5 seconds estimate
    };
    
    Ok(Json(response))
}

#[instrument(skip(state))]
async fn get_metrics(State(state): State<AppState>) -> Result<Json<MetricsResponse>, StatusCode> {
    let metrics = state.metrics.read().unwrap();
    
    // Get top strategies from database
    let top_strategies = match get_top_strategies(&state.db).await {
        Ok(strategies) => strategies,
        Err(e) => {
            warn!("Failed to get top strategies: {}", e);
            vec![]
        }
    };
    
    let active_chains: Vec<String> = state
        .opportunities
        .iter()
        .map(|entry| entry.value().chain.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    let response = MetricsResponse {
        opportunities_per_minute: metrics.opportunities_per_minute,
        executions_per_minute: metrics.executions_per_minute,
        success_rate: if metrics.total_executions > 0 {
            metrics.successful_executions as f64 / metrics.total_executions as f64
        } else {
            0.0
        },
        avg_profit_usd: if metrics.successful_executions > 0 {
            metrics.total_profit_usd / metrics.successful_executions as f64
        } else {
            0.0
        },
        avg_gas_cost_usd: if metrics.total_executions > 0 {
            metrics.total_gas_cost_usd / metrics.total_executions as f64
        } else {
            0.0
        },
        total_volume_usd: metrics.total_profit_usd,
        active_chains,
        top_strategies,
    };
    
    Ok(Json(response))
}

// Database operations
async fn store_execution(
    db: &PgPool,
    execution_id: &str,
    opportunity: &Opportunity,
    request: &ExecutionRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO executions (
            id, opportunity_id, strategy, status, max_gas_price_gwei,
            slippage_tolerance, deadline_seconds, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        execution_id,
        opportunity.id,
        request.strategy,
        "submitted",
        request.max_gas_price_gwei.map(|x| x as i32),
        request.slippage_tolerance,
        request.deadline_seconds.map(|x| x as i32),
        Utc::now()
    )
    .execute(db)
    .await?;
    
    Ok(())
}

async fn get_top_strategies(db: &PgPool) -> Result<Vec<StrategyMetric>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            o.opportunity_type as name,
            COUNT(*) as count,
            AVG(CASE WHEN e.success THEN 1.0 ELSE 0.0 END) as success_rate,
            AVG(e.profit_actual_usd) as avg_profit_usd
        FROM opportunities o
        LEFT JOIN executions e ON o.id = e.opportunity_id
        WHERE o.created_at > NOW() - INTERVAL '24 hours'
        GROUP BY o.opportunity_type
        ORDER BY count DESC
        LIMIT 10
        "#
    )
    .fetch_all(db)
    .await?;
    
    let strategies = rows
        .into_iter()
        .map(|row| StrategyMetric {
            name: row.name,
            count: row.count.unwrap_or(0) as u64,
            success_rate: row.success_rate.unwrap_or(0.0),
            avg_profit_usd: row.avg_profit_usd.unwrap_or(0.0),
        })
        .collect();
    
    Ok(strategies)
}

// Background tasks
async fn cleanup_expired_opportunities(state: AppState) {
    let mut interval = interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        let now = Utc::now();
        let mut expired_ids = Vec::new();
        
        for entry in state.opportunities.iter() {
            if entry.value().expires_at < now {
                expired_ids.push(entry.key().clone());
            }
        }
        
        for id in expired_ids {
            if let Some((_, opportunity)) = state.opportunities.remove(&id) {
                debug!("Expired opportunity: {}", opportunity.id);
                let _ = state.event_sender.send(OpportunityEvent::Expired(id));
            }
        }
    }
}

async fn update_metrics(state: AppState) {
    let mut interval = interval(Duration::from_secs(60));
    
    loop {
        interval.tick().await;
        
        // Calculate opportunities per minute
        let opportunities_count = state.opportunities.len() as f64;
        
        // Update metrics from database
        if let Ok(db_metrics) = get_database_metrics(&state.db).await {
            let mut metrics = state.metrics.write().unwrap();
            metrics.total_executions = db_metrics.total_executions;
            metrics.successful_executions = db_metrics.successful_executions;
            metrics.total_profit_usd = db_metrics.total_profit_usd;
            metrics.total_gas_cost_usd = db_metrics.total_gas_cost_usd;
            metrics.opportunities_per_minute = opportunities_count;
        }
    }
}

async fn get_database_metrics(db: &PgPool) -> Result<SystemMetrics, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total_executions,
            COUNT(CASE WHEN success THEN 1 END) as successful_executions,
            COALESCE(SUM(profit_actual_usd), 0) as total_profit_usd,
            COALESCE(SUM(gas_used * gas_price_actual_gwei * 1e-9 * 2000), 0) as total_gas_cost_usd
        FROM executions
        WHERE created_at > NOW() - INTERVAL '24 hours'
        "#
    )
    .fetch_one(db)
    .await?;
    
    Ok(SystemMetrics {
        total_opportunities: 0,
        total_executions: row.total_executions.unwrap_or(0) as u64,
        successful_executions: row.successful_executions.unwrap_or(0) as u64,
        total_profit_usd: row.total_profit_usd.unwrap_or(0.0),
        total_gas_cost_usd: row.total_gas_cost_usd.unwrap_or(0.0),
        opportunities_per_minute: 0.0,
        executions_per_minute: 0.0,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("selector_api=debug,info")
        .json()
        .init();
    
    info!("Starting ArbitrageX Supreme V3.0 Selector API");
    
    // Load configuration
    let config = Arc::new(AppConfig {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://arbitragex:password@localhost:5432/arbitragex_supreme".to_string()),
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        port: std::env::var("SELECTOR_API_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080),
        max_opportunities: 10000,
        min_profit_threshold_usd: 5.0,
        max_risk_score: 0.8,
        opportunity_ttl_seconds: 300,
    });
    
    // Initialize database connection
    let db = PgPool::connect(&config.database_url).await?;
    info!("Connected to PostgreSQL database");
    
    // Initialize Redis connection
    let redis = redis::Client::open(config.redis_url.as_str())?;
    info!("Connected to Redis");
    
    // Initialize application state
    let (event_sender, _) = broadcast::channel(1000);
    let state = AppState {
        db,
        redis,
        opportunities: Arc::new(DashMap::new()),
        metrics: Arc::new(RwLock::new(SystemMetrics {
            total_opportunities: 0,
            total_executions: 0,
            successful_executions: 0,
            total_profit_usd: 0.0,
            total_gas_cost_usd: 0.0,
            opportunities_per_minute: 0.0,
            executions_per_minute: 0.0,
        })),
        config: config.clone(),
        event_sender,
        start_time: SystemTime::now(),
    };
    
    // Start background tasks
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        cleanup_expired_opportunities(cleanup_state).await;
    });
    
    let metrics_state = state.clone();
    tokio::spawn(async move {
        update_metrics(metrics_state).await;
    });
    
    // Build router

    // Run migrations
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    // Initialize Redis
    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis_conn = Arc::new(tokio::sync::RwLock::new(
        redis_client.get_tokio_connection().await?,
    ));

    // Create app state
    let app_state = Arc::new(AppState::new(config.clone(), db_pool, redis_conn));

    // Build application
    let app = create_app(app_state).await?;

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🌐 Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn create_app(state: Arc<AppState>) -> Result<Router> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))
        
        // API routes
        .nest("/api/v1", routes::api_routes())
        
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        
        // Metrics endpoint
        .route("/metrics", get(handlers::metrics::metrics_handler))
        
        // Add middleware
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    Ok(app)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state))
}

