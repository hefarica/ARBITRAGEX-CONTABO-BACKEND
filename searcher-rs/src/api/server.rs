use anyhow::Result;
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn, error};

use crate::core::{OpportunityDetector, StateManager};
use crate::database::Database;
use crate::types::{Opportunity, Config};

#[derive(Clone)]
pub struct ApiServer {
    port: u16,
    state_manager: Arc<StateManager>,
    detector: Arc<OpportunityDetector>,
    database: Arc<Database>,
    active_opportunities: Arc<RwLock<HashMap<String, Opportunity>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpportunityQuery {
    pub chain_id: Option<u64>,
    pub strategy_type: Option<String>,
    pub min_profit: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub opportunity_id: String,
    pub max_gas_price: Option<String>,
    pub slippage_tolerance: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub status: String,
    pub uptime_seconds: u64,
    pub active_opportunities: usize,
    pub total_executions: u64,
    pub success_rate: f64,
    pub chains_monitored: Vec<u64>,
    pub strategies_active: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub opportunities_detected_1h: u64,
    pub opportunities_executed_1h: u64,
    pub total_profit_24h: String,
    pub average_profit_per_execution: String,
    pub gas_efficiency: f64,
    pub success_rate_24h: f64,
}

impl ApiServer {
    pub fn new(
        port: u16,
        state_manager: Arc<StateManager>,
        detector: Arc<OpportunityDetector>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            port,
            state_manager,
            detector,
            database,
            active_opportunities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let app = self.create_router().await;

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        
        info!("🌐 ArbitrageX API server starting on port {}", self.port);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }

    async fn create_router(&self) -> Router {
        Router::new()
            // Health and status endpoints
            .route("/health", get(health_check))
            .route("/status", get(system_status))
            .route("/metrics", get(performance_metrics))
            
            // Opportunity endpoints
            .route("/api/v1/opportunities", get(list_opportunities))
            .route("/api/v1/opportunities/:id", get(get_opportunity))
            .route("/api/v1/opportunities/:id/execute", post(execute_opportunity))
            .route("/api/v1/opportunities/simulate", post(simulate_opportunity))
            
            // Strategy endpoints
            .route("/api/v1/strategies", get(list_strategies))
            .route("/api/v1/strategies/:name/config", get(get_strategy_config))
            .route("/api/v1/strategies/:name/config", post(update_strategy_config))
            
            // Execution endpoints
            .route("/api/v1/executions", get(list_executions))
            .route("/api/v1/executions/:id", get(get_execution))
            
            // Analytics endpoints
            .route("/api/v1/analytics/performance", get(get_performance_analytics))
            .route("/api/v1/analytics/pnl", get(get_pnl_analytics))
            .route("/api/v1/analytics/risk", get(get_risk_analytics))
            
            // WebSocket endpoint
            .route("/ws", get(websocket_handler))
            
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(
                        CorsLayer::new()
                            .allow_origin(Any)
                            .allow_methods(Any)
                            .allow_headers(Any)
                    )
            )
            .with_state(self.clone())
    }
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse {
        success: true,
        data: Some("ArbitrageX MEV Engine is healthy"),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// System status endpoint
async fn system_status(State(server): State<ApiServer>) -> impl IntoResponse {
    let opportunities = server.active_opportunities.read().await;
    let chains_monitored = vec![1, 137]; // Ethereum, Polygon
    let strategies_active = vec![
        "triangular_arbitrage".to_string(),
        "flash_loan_arbitrage".to_string(),
        "cross_dex_arbitrage".to_string(),
    ];

    let status = SystemStatus {
        status: "running".to_string(),
        uptime_seconds: 3600, // TODO: Calculate actual uptime
        active_opportunities: opportunities.len(),
        total_executions: 0, // TODO: Get from database
        success_rate: 0.85, // TODO: Calculate from database
        chains_monitored,
        strategies_active,
    };

    Json(ApiResponse {
        success: true,
        data: Some(status),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// Performance metrics endpoint
async fn performance_metrics(State(server): State<ApiServer>) -> impl IntoResponse {
    // TODO: Implement real metrics from database
    let metrics = PerformanceMetrics {
        opportunities_detected_1h: 45,
        opportunities_executed_1h: 12,
        total_profit_24h: "2.5".to_string(), // ETH
        average_profit_per_execution: "0.025".to_string(), // ETH
        gas_efficiency: 0.92,
        success_rate_24h: 0.87,
    };

    Json(ApiResponse {
        success: true,
        data: Some(metrics),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// List opportunities endpoint
async fn list_opportunities(
    Query(params): Query<OpportunityQuery>,
    State(server): State<ApiServer>,
) -> impl IntoResponse {
    let opportunities = server.active_opportunities.read().await;
    
    let mut filtered_opportunities: Vec<&Opportunity> = opportunities.values().collect();
    
    // Apply filters
    if let Some(chain_id) = params.chain_id {
        filtered_opportunities.retain(|opp| opp.chain_id == chain_id);
    }
    
    if let Some(strategy_type) = &params.strategy_type {
        filtered_opportunities.retain(|opp| opp.strategy_type == *strategy_type);
    }
    
    if let Some(min_profit_str) = &params.min_profit {
        if let Ok(min_profit) = ethers::types::U256::from_dec_str(min_profit_str) {
            filtered_opportunities.retain(|opp| opp.expected_profit >= min_profit);
        }
    }
    
    // Apply limit
    if let Some(limit) = params.limit {
        filtered_opportunities.truncate(limit);
    }
    
    Json(ApiResponse {
        success: true,
        data: Some(filtered_opportunities),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// Get specific opportunity endpoint
async fn get_opportunity(
    Path(id): Path<String>,
    State(server): State<ApiServer>,
) -> impl IntoResponse {
    let opportunities = server.active_opportunities.read().await;
    
    match opportunities.get(&id) {
        Some(opportunity) => Json(ApiResponse {
            success: true,
            data: Some(opportunity),
            error: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }),
        None => {
            warn!("Opportunity not found: {}", id);
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some("Opportunity not found".to_string()),
                    timestamp: chrono::Utc::now().timestamp() as u64,
                })
            ).into_response()
        }
    }
}

// Execute opportunity endpoint
async fn execute_opportunity(
    Path(id): Path<String>,
    State(server): State<ApiServer>,
    Json(request): Json<ExecutionRequest>,
) -> impl IntoResponse {
    info!("🚀 Executing opportunity: {}", id);
    
    let opportunities = server.active_opportunities.read().await;
    
    match opportunities.get(&id) {
        Some(opportunity) => {
            // Validate opportunity is still valid
            match server.detector.validate_opportunity(opportunity).await {
                Ok(true) => {
                    // TODO: Implement actual execution logic
                    info!("✅ Opportunity {} validated for execution", id);
                    
                    Json(ApiResponse {
                        success: true,
                        data: Some("Execution initiated"),
                        error: None,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    })
                },
                Ok(false) => {
                    warn!("❌ Opportunity {} is no longer valid", id);
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::<()> {
                            success: false,
                            data: None,
                            error: Some("Opportunity is no longer valid".to_string()),
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        })
                    ).into_response()
                },
                Err(e) => {
                    error!("Error validating opportunity {}: {}", id, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()> {
                            success: false,
                            data: None,
                            error: Some("Validation error".to_string()),
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        })
                    ).into_response()
                }
            }
        },
        None => {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some("Opportunity not found".to_string()),
                    timestamp: chrono::Utc::now().timestamp() as u64,
                })
            ).into_response()
        }
    }
}

// Simulate opportunity endpoint
async fn simulate_opportunity(
    State(server): State<ApiServer>,
    Json(opportunity): Json<Opportunity>,
) -> impl IntoResponse {
    info!("🧪 Simulating opportunity: {}", opportunity.id);
    
    // TODO: Implement simulation logic using sim-ctl
    Json(ApiResponse {
        success: true,
        data: Some("Simulation completed"),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// List strategies endpoint
async fn list_strategies(State(server): State<ApiServer>) -> impl IntoResponse {
    let strategies = vec![
        "triangular_arbitrage",
        "flash_loan_arbitrage", 
        "cross_dex_arbitrage",
        "sandwich_attack",
        "liquidation",
    ];
    
    Json(ApiResponse {
        success: true,
        data: Some(strategies),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// Get strategy config endpoint
async fn get_strategy_config(
    Path(name): Path<String>,
    State(server): State<ApiServer>,
) -> impl IntoResponse {
    // TODO: Get actual strategy configuration
    let config = serde_json::json!({
        "name": name,
        "enabled": true,
        "min_profit_wei": "10000000000000000",
        "max_gas_price": "100000000000",
        "parameters": {}
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(config),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// Update strategy config endpoint
async fn update_strategy_config(
    Path(name): Path<String>,
    State(server): State<ApiServer>,
    Json(config): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("⚙️ Updating strategy config: {}", name);
    
    // TODO: Implement actual config update
    Json(ApiResponse {
        success: true,
        data: Some("Configuration updated"),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// List executions endpoint
async fn list_executions(State(server): State<ApiServer>) -> impl IntoResponse {
    // TODO: Get executions from database
    let executions: Vec<serde_json::Value> = vec![];
    
    Json(ApiResponse {
        success: true,
        data: Some(executions),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// Get execution endpoint
async fn get_execution(
    Path(id): Path<String>,
    State(server): State<ApiServer>,
) -> impl IntoResponse {
    // TODO: Get execution from database
    Json(ApiResponse {
        success: true,
        data: Some(format!("Execution {}", id)),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// Performance analytics endpoint
async fn get_performance_analytics(State(server): State<ApiServer>) -> impl IntoResponse {
    // TODO: Calculate real analytics from database
    let analytics = serde_json::json!({
        "total_opportunities": 1250,
        "successful_executions": 1087,
        "total_profit_eth": "45.7",
        "average_roi": 0.034,
        "best_strategy": "triangular_arbitrage"
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(analytics),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// P&L analytics endpoint
async fn get_pnl_analytics(State(server): State<ApiServer>) -> impl IntoResponse {
    // TODO: Calculate real P&L from database
    let pnl = serde_json::json!({
        "daily_pnl": "2.3",
        "weekly_pnl": "15.8",
        "monthly_pnl": "67.2",
        "total_pnl": "234.5",
        "currency": "ETH"
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(pnl),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// Risk analytics endpoint
async fn get_risk_analytics(State(server): State<ApiServer>) -> impl IntoResponse {
    // TODO: Calculate real risk metrics
    let risk = serde_json::json!({
        "var_95": "0.45",
        "max_drawdown": "0.12",
        "sharpe_ratio": 2.34,
        "success_rate": 0.87,
        "average_gas_efficiency": 0.92
    });
    
    Json(ApiResponse {
        success: true,
        data: Some(risk),
        error: None,
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(server): State<ApiServer>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, server))
}

async fn handle_websocket(
    mut socket: axum::extract::ws::WebSocket,
    server: ApiServer,
) {
    info!("🔌 New WebSocket connection established");
    
    // TODO: Implement real-time data streaming
    // This would stream opportunities, executions, and market data
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        let message = serde_json::json!({
            "type": "heartbeat",
            "timestamp": chrono::Utc::now().timestamp()
        });
        
        if socket.send(axum::extract::ws::Message::Text(message.to_string())).await.is_err() {
            break;
        }
    }
    
    info!("🔌 WebSocket connection closed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
