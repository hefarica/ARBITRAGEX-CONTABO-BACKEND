use anyhow::Result;
use ethers::types::{Transaction, U256, H256, Address, Bytes};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdenBundle {
    pub id: String,
    pub transactions: Vec<String>, // Raw signed transactions
    pub block_number: u64,
    pub max_block_number: Option<u64>,
    pub min_timestamp: Option<u64>,
    pub max_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdenBundleResponse {
    pub bundle_hash: H256,
    pub block_number: u64,
    pub inclusion_status: String,
    pub gas_price: String,
    pub gas_used: u64,
    pub effective_gas_price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdenSimulation {
    pub bundle_hash: H256,
    pub success: bool,
    pub gas_used: u64,
    pub gas_price: String,
    pub coinbase_diff: String,
    pub transactions: Vec<EdenTransactionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdenTransactionResult {
    pub tx_hash: H256,
    pub success: bool,
    pub gas_used: u64,
    pub gas_price: String,
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub value: String,
    pub logs: Vec<serde_json::Value>,
}

pub struct EdenClient {
    client: Client,
    relay_url: String,
    api_key: String,
    max_retries: u32,
    timeout_seconds: u64,
}

impl EdenClient {
    pub fn new(relay_url: String, api_key: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("ArbitrageX-Supreme/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            relay_url,
            api_key,
            max_retries: 3,
            timeout_seconds: 30,
        }
    }

    /// Submit a bundle to Eden Network
    pub async fn submit_bundle(&self, bundle: &EdenBundle) -> Result<EdenBundleResponse> {
        info!("🌿 Submitting bundle {} to Eden Network for block {}", bundle.id, bundle.block_number);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eden_sendBundle",
            "params": [{
                "txs": bundle.transactions,
                "blockNumber": format!("0x{:x}", bundle.block_number),
                "maxBlockNumber": bundle.max_block_number.map(|n| format!("0x{:x}", n)),
                "minTimestamp": bundle.min_timestamp,
                "maxTimestamp": bundle.max_timestamp
            }]
        });

        let mut retries = 0;
        loop {
            match self.send_request(&payload).await {
                Ok(response) => {
                    info!("✅ Bundle {} submitted successfully to Eden", bundle.id);
                    return self.parse_bundle_response(response, bundle.block_number);
                },
                Err(e) => {
                    retries += 1;
                    if retries >= self.max_retries {
                        error!("❌ Failed to submit bundle {} to Eden after {} retries: {}", bundle.id, retries, e);
                        return Err(e);
                    }
                    warn!("⚠️ Eden bundle submission failed, retrying ({}/{}): {}", retries, self.max_retries, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }
    }

    /// Simulate a bundle on Eden Network
    pub async fn simulate_bundle(&self, bundle: &EdenBundle) -> Result<EdenSimulation> {
        debug!("🧪 Simulating bundle {} on Eden for block {}", bundle.id, bundle.block_number);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eden_callBundle",
            "params": [{
                "txs": bundle.transactions,
                "blockNumber": format!("0x{:x}", bundle.block_number),
                "stateBlockNumber": "latest"
            }]
        });

        let response = self.send_request(&payload).await?;
        self.parse_simulation_response(response)
    }

    /// Send a single transaction via Eden's private mempool
    pub async fn send_private_transaction(&self, signed_tx: String) -> Result<H256> {
        info!("🔒 Sending private transaction via Eden Network");

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eden_sendRawTransaction",
            "params": [signed_tx]
        });

        let response = self.send_request(&payload).await?;
        
        let tx_hash = response.get("result")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid transaction hash in response"))?;

        info!("✅ Private transaction sent: {:?}", tx_hash);
        Ok(tx_hash)
    }

    /// Get bundle inclusion status
    pub async fn get_bundle_status(&self, bundle_hash: H256) -> Result<serde_json::Value> {
        debug!("📊 Getting bundle status for {:?}", bundle_hash);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eden_getBundleStatus",
            "params": [format!("{:?}", bundle_hash)]
        });

        let response = self.send_request(&payload).await?;
        Ok(response)
    }

    /// Get Eden Network statistics
    pub async fn get_network_stats(&self) -> Result<serde_json::Value> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eden_getNetworkStats",
            "params": []
        });

        let response = self.send_request(&payload).await?;
        Ok(response)
    }

    /// Cancel a pending bundle
    pub async fn cancel_bundle(&self, bundle_hash: H256) -> Result<bool> {
        info!("❌ Cancelling bundle {:?}", bundle_hash);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eden_cancelBundle",
            "params": [format!("{:?}", bundle_hash)]
        });

        let response = self.send_request(&payload).await?;
        
        let success = response.get("result")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if success {
            info!("✅ Bundle {:?} cancelled successfully", bundle_hash);
        } else {
            warn!("⚠️ Failed to cancel bundle {:?}", bundle_hash);
        }

        Ok(success)
    }

    /// Send authenticated request to Eden Network
    async fn send_request(&self, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let body = serde_json::to_string(payload)?;

        let response = self.client
            .post(&self.relay_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Eden-Key", &self.api_key)
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("HTTP error {}: {}", response.status(), error_text));
        }

        let json: serde_json::Value = response.json().await?;
        
        if let Some(error) = json.get("error") {
            return Err(anyhow::anyhow!("RPC error: {}", error));
        }

        Ok(json)
    }

    /// Parse bundle submission response
    fn parse_bundle_response(&self, response: serde_json::Value, block_number: u64) -> Result<EdenBundleResponse> {
        let result = response.get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

        let bundle_hash = result.get("bundleHash")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| H256::random());

        Ok(EdenBundleResponse {
            bundle_hash,
            block_number,
            inclusion_status: "pending".to_string(),
            gas_price: "0".to_string(),
            gas_used: 0,
            effective_gas_price: "0".to_string(),
        })
    }

    /// Parse simulation response
    fn parse_simulation_response(&self, response: serde_json::Value) -> Result<EdenSimulation> {
        let result = response.get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in simulation response"))?;

        let bundle_hash = H256::random();
        let success = result.get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let gas_used = result.get("gasUsed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let transactions = result.get("transactions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|tx| {
                    Some(EdenTransactionResult {
                        tx_hash: tx.get("txHash")?.as_str()?.parse().ok()?,
                        success: tx.get("success")?.as_bool()?,
                        gas_used: tx.get("gasUsed")?.as_u64()?,
                        gas_price: tx.get("gasPrice")?.as_str()?.to_string(),
                        from_address: tx.get("from")?.as_str()?.parse().ok()?,
                        to_address: tx.get("to")?.as_str()?.parse().ok(),
                        value: tx.get("value")?.as_str()?.to_string(),
                        logs: tx.get("logs")?.as_array()?.clone().unwrap_or_default(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Ok(EdenSimulation {
            bundle_hash,
            success,
            gas_used,
            gas_price: "0".to_string(),
            coinbase_diff: "0".to_string(),
            transactions,
        })
    }
}

impl EdenBundle {
    pub fn new(block_number: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            transactions: Vec::new(),
            block_number,
            max_block_number: None,
            min_timestamp: None,
            max_timestamp: None,
        }
    }

    pub fn add_transaction(&mut self, signed_tx: String) {
        self.transactions.push(signed_tx);
    }

    pub fn set_block_range(&mut self, max_block_number: Option<u64>) {
        self.max_block_number = max_block_number;
    }

    pub fn set_timing(&mut self, min_timestamp: Option<u64>, max_timestamp: Option<u64>) {
        self.min_timestamp = min_timestamp;
        self.max_timestamp = max_timestamp;
    }

    pub fn is_valid(&self) -> bool {
        !self.transactions.is_empty() && self.block_number > 0
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_eden_bundle_creation() {
        let mut bundle = EdenBundle::new(12345);
        bundle.add_transaction("0x1234".to_string());
        bundle.set_block_range(Some(12350));
        
        assert!(bundle.is_valid());
        assert_eq!(bundle.transaction_count(), 1);
        assert_eq!(bundle.block_number, 12345);
        assert_eq!(bundle.max_block_number, Some(12350));
    }

    #[test]
    fn test_eden_client_creation() {
        let client = EdenClient::new(
            "https://api.edennetwork.io/v1".to_string(),
            "test_api_key".to_string(),
        );
        
        assert_eq!(client.relay_url, "https://api.edennetwork.io/v1");
        assert_eq!(client.max_retries, 3);
    }
}
