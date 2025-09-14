use anyhow::Result;
use ethers::types::{Transaction, U256, H256, Address};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::flashbots::{FlashbotsClient, FlashbotsBundle, FlashbotsBundleResponse};
use crate::eden::{EdenClient, EdenBundle, EdenBundleResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelayType {
    Flashbots,
    Eden,
    Manifold,
    BeaverBuild,
    Titan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub relay_type: RelayType,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub signing_key: Option<String>,
    pub enabled: bool,
    pub priority: u8, // 1 = highest priority
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSubmissionRequest {
    pub id: String,
    pub transactions: Vec<String>,
    pub block_number: u64,
    pub max_block_number: Option<u64>,
    pub min_timestamp: Option<u64>,
    pub max_timestamp: Option<u64>,
    pub target_relays: Vec<RelayType>,
    pub simulation_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSubmissionResult {
    pub bundle_id: String,
    pub relay_results: HashMap<RelayType, RelaySubmissionResult>,
    pub overall_success: bool,
    pub best_relay: Option<RelayType>,
    pub total_gas_used: u64,
    pub estimated_profit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelaySubmissionResult {
    pub success: bool,
    pub bundle_hash: Option<H256>,
    pub error: Option<String>,
    pub gas_used: u64,
    pub gas_price: String,
    pub simulation_success: bool,
    pub coinbase_diff: String,
    pub submission_time_ms: u64,
}

pub struct RelayManager {
    flashbots_client: Option<FlashbotsClient>,
    eden_client: Option<EdenClient>,
    relay_configs: HashMap<RelayType, RelayConfig>,
    default_timeout: u64,
}

impl RelayManager {
    pub fn new() -> Self {
        Self {
            flashbots_client: None,
            eden_client: None,
            relay_configs: HashMap::new(),
            default_timeout: 30,
        }
    }

    /// Initialize relay clients based on configuration
    pub fn initialize(&mut self, configs: Vec<RelayConfig>) -> Result<()> {
        info!("🔗 Initializing relay clients...");

        for config in configs {
            self.relay_configs.insert(config.relay_type.clone(), config.clone());

            match config.relay_type {
                RelayType::Flashbots => {
                    if config.enabled {
                        let signing_key = config.signing_key
                            .ok_or_else(|| anyhow::anyhow!("Flashbots requires signing key"))?;
                        
                        self.flashbots_client = Some(FlashbotsClient::new(
                            config.endpoint,
                            signing_key,
                        ));
                        info!("✅ Flashbots client initialized");
                    }
                },
                RelayType::Eden => {
                    if config.enabled {
                        let api_key = config.api_key
                            .ok_or_else(|| anyhow::anyhow!("Eden requires API key"))?;
                        
                        self.eden_client = Some(EdenClient::new(
                            config.endpoint,
                            api_key,
                        ));
                        info!("✅ Eden client initialized");
                    }
                },
                _ => {
                    warn!("⚠️ Relay type {:?} not yet implemented", config.relay_type);
                }
            }
        }

        Ok(())
    }

    /// Submit bundle to multiple relays simultaneously
    pub async fn submit_bundle(&self, request: &BundleSubmissionRequest) -> Result<BundleSubmissionResult> {
        info!("🚀 Submitting bundle {} to {} relays", request.id, request.target_relays.len());

        let mut relay_results = HashMap::new();
        let mut tasks = Vec::new();

        // Create submission tasks for each target relay
        for relay_type in &request.target_relays {
            if let Some(config) = self.relay_configs.get(relay_type) {
                if config.enabled {
                    let task = self.submit_to_relay(relay_type.clone(), request).await;
                    match task {
                        Ok(result) => {
                            relay_results.insert(relay_type.clone(), result);
                        },
                        Err(e) => {
                            error!("❌ Failed to submit to {:?}: {}", relay_type, e);
                            relay_results.insert(relay_type.clone(), RelaySubmissionResult {
                                success: false,
                                bundle_hash: None,
                                error: Some(e.to_string()),
                                gas_used: 0,
                                gas_price: "0".to_string(),
                                simulation_success: false,
                                coinbase_diff: "0".to_string(),
                                submission_time_ms: 0,
                            });
                        }
                    }
                }
            }
        }

        // Analyze results
        let overall_success = relay_results.values().any(|r| r.success);
        let best_relay = self.find_best_relay(&relay_results);
        let total_gas_used = relay_results.values().map(|r| r.gas_used).max().unwrap_or(0);

        let result = BundleSubmissionResult {
            bundle_id: request.id.clone(),
            relay_results,
            overall_success,
            best_relay,
            total_gas_used,
            estimated_profit: "0".to_string(), // Would be calculated based on simulation
        };

        if result.overall_success {
            info!("✅ Bundle {} submitted successfully to at least one relay", request.id);
        } else {
            error!("❌ Bundle {} failed to submit to any relay", request.id);
        }

        Ok(result)
    }

    /// Submit to a specific relay
    async fn submit_to_relay(&self, relay_type: RelayType, request: &BundleSubmissionRequest) -> Result<RelaySubmissionResult> {
        let start_time = std::time::Instant::now();

        let result = match relay_type {
            RelayType::Flashbots => {
                if let Some(client) = &self.flashbots_client {
                    self.submit_to_flashbots(client, request).await?
                } else {
                    return Err(anyhow::anyhow!("Flashbots client not initialized"));
                }
            },
            RelayType::Eden => {
                if let Some(client) = &self.eden_client {
                    self.submit_to_eden(client, request).await?
                } else {
                    return Err(anyhow::anyhow!("Eden client not initialized"));
                }
            },
            _ => {
                return Err(anyhow::anyhow!("Relay type {:?} not implemented", relay_type));
            }
        };

        let submission_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(RelaySubmissionResult {
            success: true,
            bundle_hash: Some(result.0),
            error: None,
            gas_used: result.1,
            gas_price: result.2,
            simulation_success: result.3,
            coinbase_diff: result.4,
            submission_time_ms,
        })
    }

    /// Submit to Flashbots
    async fn submit_to_flashbots(&self, client: &FlashbotsClient, request: &BundleSubmissionRequest) -> Result<(H256, u64, String, bool, String)> {
        let mut bundle = FlashbotsBundle::new(request.block_number);
        bundle.id = request.id.clone();
        
        for tx in &request.transactions {
            bundle.add_transaction(tx.clone());
        }
        
        bundle.set_timing(request.min_timestamp, request.max_timestamp);

        // Simulate first if required
        if request.simulation_required {
            let simulation = client.simulate_bundle(&bundle).await?;
            if simulation.first_revert.is_some() {
                return Err(anyhow::anyhow!("Bundle simulation failed"));
            }
        }

        let response = client.submit_bundle(&bundle).await?;
        
        Ok((
            response.bundle_hash,
            response.gas_used,
            response.gas_price,
            response.simulation_success,
            response.coinbase_diff,
        ))
    }

    /// Submit to Eden Network
    async fn submit_to_eden(&self, client: &EdenClient, request: &BundleSubmissionRequest) -> Result<(H256, u64, String, bool, String)> {
        let mut bundle = EdenBundle::new(request.block_number);
        bundle.id = request.id.clone();
        
        for tx in &request.transactions {
            bundle.add_transaction(tx.clone());
        }
        
        bundle.set_block_range(request.max_block_number);
        bundle.set_timing(request.min_timestamp, request.max_timestamp);

        // Simulate first if required
        if request.simulation_required {
            let simulation = client.simulate_bundle(&bundle).await?;
            if !simulation.success {
                return Err(anyhow::anyhow!("Eden bundle simulation failed"));
            }
        }

        let response = client.submit_bundle(&bundle).await?;
        
        Ok((
            response.bundle_hash,
            response.gas_used,
            response.gas_price,
            true, // Eden doesn't return simulation status in submission
            "0".to_string(), // Eden doesn't return coinbase diff
        ))
    }

    /// Find the best relay based on results
    fn find_best_relay(&self, results: &HashMap<RelayType, RelaySubmissionResult>) -> Option<RelayType> {
        let mut best_relay = None;
        let mut best_score = 0.0;

        for (relay_type, result) in results {
            if result.success {
                let config = self.relay_configs.get(relay_type)?;
                
                // Calculate score based on priority, speed, and success
                let mut score = 100.0 - (config.priority as f64 * 10.0); // Lower priority number = higher score
                score += if result.simulation_success { 20.0 } else { 0.0 };
                score -= result.submission_time_ms as f64 / 1000.0; // Faster = better
                
                if score > best_score {
                    best_score = score;
                    best_relay = Some(relay_type.clone());
                }
            }
        }

        best_relay
    }

    /// Get relay statistics
    pub async fn get_relay_stats(&self) -> Result<HashMap<RelayType, serde_json::Value>> {
        let mut stats = HashMap::new();

        if let Some(client) = &self.flashbots_client {
            if let Ok(flashbots_stats) = client.get_user_stats(0).await {
                stats.insert(RelayType::Flashbots, flashbots_stats);
            }
        }

        if let Some(client) = &self.eden_client {
            if let Ok(eden_stats) = client.get_network_stats().await {
                stats.insert(RelayType::Eden, eden_stats);
            }
        }

        Ok(stats)
    }

    /// Health check for all relays
    pub async fn health_check(&self) -> HashMap<RelayType, bool> {
        let mut health = HashMap::new();

        // Check Flashbots
        if let Some(_client) = &self.flashbots_client {
            health.insert(RelayType::Flashbots, true); // Would do actual health check
        }

        // Check Eden
        if let Some(_client) = &self.eden_client {
            health.insert(RelayType::Eden, true); // Would do actual health check
        }

        health
    }
}

impl Default for RelayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BundleSubmissionRequest {
    pub fn new(block_number: u64, transactions: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            transactions,
            block_number,
            max_block_number: None,
            min_timestamp: None,
            max_timestamp: None,
            target_relays: vec![RelayType::Flashbots, RelayType::Eden],
            simulation_required: true,
        }
    }

    pub fn with_relays(mut self, relays: Vec<RelayType>) -> Self {
        self.target_relays = relays;
        self
    }

    pub fn with_timing(mut self, min_timestamp: Option<u64>, max_timestamp: Option<u64>) -> Self {
        self.min_timestamp = min_timestamp;
        self.max_timestamp = max_timestamp;
        self
    }

    pub fn with_simulation(mut self, required: bool) -> Self {
        self.simulation_required = required;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_submission_request_creation() {
        let request = BundleSubmissionRequest::new(
            12345,
            vec!["0x1234".to_string()],
        );
        
        assert_eq!(request.block_number, 12345);
        assert_eq!(request.transactions.len(), 1);
        assert!(request.simulation_required);
    }

    #[test]
    fn test_relay_manager_creation() {
        let manager = RelayManager::new();
        assert!(manager.flashbots_client.is_none());
        assert!(manager.eden_client.is_none());
        assert_eq!(manager.default_timeout, 30);
    }

    #[test]
    fn test_relay_config() {
        let config = RelayConfig {
            relay_type: RelayType::Flashbots,
            endpoint: "https://relay.flashbots.net".to_string(),
            api_key: None,
            signing_key: Some("test_key".to_string()),
            enabled: true,
            priority: 1,
            timeout_seconds: 30,
            max_retries: 3,
        };
        
        assert!(config.enabled);
        assert_eq!(config.priority, 1);
        assert!(config.signing_key.is_some());
    }
}
