use anyhow::Result;
use std::env;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{info, error, warn};
use tracing_subscriber;
use redis::AsyncCommands;
use serde_json;

mod api;
mod eden;
mod flashbots;

use api::{RelayManager, RelayConfig, RelayType, BundleSubmissionRequest, BundleSubmissionResponse};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter("relays_client=debug,info")
        .json()
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    info!("🚀 Starting ArbitrageX Relays Client v1.0");

    // Load configuration from environment
    let config = load_config()?;
    info!("📋 Loaded {} relay configurations", config.len());
    
    // Initialize relay manager
    let mut relay_manager = RelayManager::new();
    relay_manager.initialize(config).await?;
    info!("✅ Relay manager initialized");

    // Initialize Redis connection for communication with searcher
    let redis_client = redis::Client::open(
    };
    
    // Initialize relay client
    let client = RelayClient::new(config.clone());
    
    // Start background metrics worker
    let worker_state = client.state.clone();
    tokio::spawn(async move {
        metrics_worker(worker_state).await;
    });
    
    info!("MEV Relay Client started on port {}", config.port);
    
    // Example bundle submission for testing
    let test_bundle = Bundle {
        id: Uuid::new_v4().to_string(),
        transactions: vec![
            "0x02f8b20182012a8459682f008459682f0e8301388094a0b86a33e6ba3e6f1b5f6b5a5c5b5d5e5f5g5h5i5j80b844a9059cbb000000000000000000000000742d35cc6634c0532925a3b8d3ac6d9b5c5e5f5g000000000000000000000000000000000000000000000000000000000000000ac080a0".to_string(),
        ],
        block_number: 18500000,
        min_timestamp: None,
        max_timestamp: None,
        replacement_uuid: None,
        signing_address: None,
        refund_percent: Some(90),
        refund_recipient: None,
        refund_index: None,
                        if let Err(e) = tx_responses.send(response).await {
                            error!("Failed to send response: {}", e);
                        }
                    },
                    Err(e) => {
                        error!("❌ Bundle {} submission failed: {}", request.bundle_id, e);
                        
                        let error_response = BundleSubmissionResponse {
                            bundle_id: request.bundle_id.clone(),
                            success: false,
                            relay_responses: vec![],
                            error: Some(e.to_string()),
                            submitted_at: chrono::Utc::now(),
                            block_number: request.target_block,
                        };
                        
                        if let Err(e) = tx_responses.send(error_response).await {
                            error!("Failed to send error response: {}", e);
                        }
                    }
                }
            }
        })
    };

    // Task 3: Handle responses and publish to Redis
    let response_handler_task = {
        let mut redis_conn = redis_conn.clone();
        
        tokio::spawn(async move {
            info!("📤 Starting response handler");
            
            while let Some(response) = rx_responses.recv().await {
                let response_json = match serde_json::to_string(&response) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize response: {}", e);
                        continue;
                    }
                };
                
                // Publish response to Redis
                if let Err(e) = redis_conn.rpush::<_, _, ()>("bundle_responses", &response_json).await {
                    error!("Failed to publish response to Redis: {}", e);
                } else {
                    info!("📨 Published response for bundle: {}", response.bundle_id);
                }
                
                // Also publish to specific channel for real-time updates
                let channel = format!("bundle_response:{}", response.bundle_id);
                if let Err(e) = redis_conn.publish::<_, _, ()>(&channel, &response_json).await {
                    error!("Failed to publish to channel {}: {}", channel, e);
                }
            }
        })
    };

    // Task 4: Health monitoring
    let health_monitor_task = {
        let relay_manager = relay_manager.clone();
        let mut redis_conn = redis_conn.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60));
            info!("🏥 Starting health monitor");
            
            loop {
                interval.tick().await;
                
                let health_status = relay_manager.health_check().await;
                info!("🏥 Relay health check: {:?}", health_status);
                
                // Publish health status to Redis
                let health_json = match serde_json::to_string(&health_status) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize health status: {}", e);
                        continue;
                    }
                };
                
                if let Err(e) = redis_conn.set::<_, _, ()>("relays_health", &health_json).await {
                    error!("Failed to publish health status: {}", e);
                }
                
                // Set TTL for health status
                if let Err(e) = redis_conn.expire::<_, ()>("relays_health", 120).await {
                    error!("Failed to set TTL for health status: {}", e);
                }
            }
        })
    };

    // Task 5: Metrics collection
    let metrics_task = {
        let relay_manager = relay_manager.clone();
        let mut redis_conn = redis_conn.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // Every 5 minutes
            info!("📊 Starting metrics collector");
            
            loop {
                interval.tick().await;
                
                let metrics = relay_manager.get_metrics().await;
                info!("📊 Collected metrics: {:?}", metrics);
                
                let metrics_json = match serde_json::to_string(&metrics) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize metrics: {}", e);
                        continue;
                    }
                };
                
                // Store metrics with timestamp
                let timestamp = chrono::Utc::now().timestamp();
                let key = format!("relays_metrics:{}", timestamp);
                
                if let Err(e) = redis_conn.set::<_, _, ()>(&key, &metrics_json).await {
                    error!("Failed to store metrics: {}", e);
                }
                
                // Set TTL for metrics (keep for 24 hours)
                if let Err(e) = redis_conn.expire::<_, ()>(&key, 86400).await {
                    error!("Failed to set TTL for metrics: {}", e);
                }
            }
        })
    };

    info!("✅ All tasks started successfully");

    // Wait for all tasks to complete (they should run indefinitely)
    tokio::select! {
        result = redis_listener_task => {
            error!("Redis listener task ended: {:?}", result);
        }
        result = bundle_processor_task => {
            error!("Bundle processor task ended: {:?}", result);
        }
        result = response_handler_task => {
            error!("Response handler task ended: {:?}", result);
        }
        result = health_monitor_task => {
            error!("Health monitor task ended: {:?}", result);
        }
        result = metrics_task => {
            error!("Metrics task ended: {:?}", result);
        }
    }

    warn!("🛑 Relays client shutting down");
    Ok(())
}

fn load_config() -> Result<Vec<RelayConfig>> {
    let mut configs = Vec::new();

    // Flashbots configuration
    if let Ok(flashbots_endpoint) = env::var("FLASHBOTS_RELAY_URL") {
        if let Ok(signing_key) = env::var("FLASHBOTS_SIGNING_KEY") {
            configs.push(RelayConfig {
                relay_type: RelayType::Flashbots,
                endpoint: flashbots_endpoint,
                api_key: None,
                signing_key: Some(signing_key),
                enabled: env::var("FLASHBOTS_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true",
                priority: env::var("FLASHBOTS_PRIORITY")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .unwrap_or(1),
                timeout_seconds: 30,
                max_retries: 3,
            });
        }
    }

    // Eden Network configuration
    if let Ok(eden_endpoint) = env::var("EDEN_RELAY_URL") {
        if let Ok(api_key) = env::var("EDEN_API_KEY") {
            configs.push(RelayConfig {
                relay_type: RelayType::Eden,
                endpoint: eden_endpoint,
                api_key: Some(api_key),
                signing_key: None,
                enabled: env::var("EDEN_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true",
                priority: env::var("EDEN_PRIORITY")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .unwrap_or(2),
                timeout_seconds: 30,
                max_retries: 3,
            });
        }
    }

    if configs.is_empty() {
        return Err(anyhow::anyhow!("No relay configurations found. Please set environment variables."));
    }

    info!("📋 Loaded {} relay configurations", configs.len());
    Ok(configs)
}
