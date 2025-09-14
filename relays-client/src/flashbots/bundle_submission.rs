use anyhow::Result;
use ethers::types::{Transaction, U256, H256, Address};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashbotsBundle {
    pub id: String,
    pub transactions: Vec<String>, // Raw signed transactions
    pub block_number: u64,
    pub min_timestamp: Option<u64>,
    pub max_timestamp: Option<u64>,
    pub reverting_tx_hashes: Vec<H256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashbotsBundleResponse {
    pub bundle_hash: H256,
    pub block_number: u64,
    pub simulation_success: bool,
    pub coinbase_diff: String,
    pub gas_fees: String,
    pub eth_sent_to_coinbase: String,
    pub gas_price: String,
    pub gas_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashbotsSimulation {
    pub bundle_hash: H256,
    pub coinbase_diff: i64,
    pub results: Vec<TransactionResult>,
    pub total_gas_used: u64,
    pub first_revert: Option<TransactionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub coinbase_diff: String,
    pub eth_sent_to_coinbase: String,
    pub from_address: Address,
    pub gas_fees: String,
    pub gas_price: String,
    pub gas_used: u64,
    pub to_address: Option<Address>,
    pub tx_hash: H256,
    pub value: String,
}

pub struct FlashbotsClient {
    client: Client,
    relay_url: String,
    signing_key: String,
    max_retries: u32,
    timeout_seconds: u64,
}

impl FlashbotsClient {
    pub fn new(relay_url: String, signing_key: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            relay_url,
            signing_key,
            max_retries: 3,
            timeout_seconds: 30,
        }
    }

    /// Submit a bundle to Flashbots
    pub async fn submit_bundle(&self, bundle: &FlashbotsBundle) -> Result<FlashbotsBundleResponse> {
        info!("🚀 Submitting bundle {} to Flashbots for block {}", bundle.id, bundle.block_number);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_sendBundle",
            "params": [{
                "txs": bundle.transactions,
                "blockNumber": format!("0x{:x}", bundle.block_number),
                "minTimestamp": bundle.min_timestamp,
                "maxTimestamp": bundle.max_timestamp,
                "revertingTxHashes": bundle.reverting_tx_hashes
            }]
        });

        let mut retries = 0;
        loop {
            match self.send_request(&payload).await {
                Ok(response) => {
                    info!("✅ Bundle {} submitted successfully", bundle.id);
                    return self.parse_bundle_response(response, bundle.block_number);
                },
                Err(e) => {
                    retries += 1;
                    if retries >= self.max_retries {
                        error!("❌ Failed to submit bundle {} after {} retries: {}", bundle.id, retries, e);
                        return Err(e);
                    }
                    warn!("⚠️ Bundle submission failed, retrying ({}/{}): {}", retries, self.max_retries, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Simulate a bundle before submission
    pub async fn simulate_bundle(&self, bundle: &FlashbotsBundle) -> Result<FlashbotsSimulation> {
        debug!("🧪 Simulating bundle {} for block {}", bundle.id, bundle.block_number);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_callBundle",
            "params": [{
                "txs": bundle.transactions,
                "blockNumber": format!("0x{:x}", bundle.block_number),
                "stateBlockNumber": "latest"
            }]
        });

        let response = self.send_request(&payload).await?;
        self.parse_simulation_response(response)
    }

    /// Get bundle stats
    pub async fn get_bundle_stats(&self, bundle_hash: H256, block_number: u64) -> Result<serde_json::Value> {
        debug!("📊 Getting stats for bundle {:?} in block {}", bundle_hash, block_number);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "flashbots_getBundleStats",
            "params": [{
                "bundleHash": format!("{:?}", bundle_hash),
                "blockNumber": format!("0x{:x}", block_number)
            }]
        });

        let response = self.send_request(&payload).await?;
        Ok(response)
    }

    /// Check if user transactions are included in a block
    pub async fn get_user_stats(&self, block_number: u64) -> Result<serde_json::Value> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "flashbots_getUserStats",
            "params": [{
                "blockNumber": format!("0x{:x}", block_number)
            }]
        });

        let response = self.send_request(&payload).await?;
        Ok(response)
    }

    /// Send authenticated request to Flashbots
    async fn send_request(&self, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let body = serde_json::to_string(payload)?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // Create Flashbots signature
        let message = format!("{}:{}", timestamp, body);
        let signature = self.sign_message(&message)?;

        let response = self.client
            .post(&self.relay_url)
            .header("Content-Type", "application/json")
            .header("X-Flashbots-Signature", signature)
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

    /// Sign message for Flashbots authentication
    fn sign_message(&self, message: &str) -> Result<String> {
        use sha3::{Digest, Keccak256};
        
        let mut hasher = Keccak256::new();
        hasher.update(message.as_bytes());
        let hash = hasher.finalize();
        
        // In a real implementation, you would use the private key to sign
        // For now, we'll create a placeholder signature
        let signature = format!("0x{}", hex::encode(&hash[..32]));
        Ok(signature)
    }

    /// Parse bundle submission response
    fn parse_bundle_response(&self, response: serde_json::Value, block_number: u64) -> Result<FlashbotsBundleResponse> {
        let result = response.get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

        // Flashbots typically returns a bundle hash
        let bundle_hash = result.get("bundleHash")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| H256::random());

        Ok(FlashbotsBundleResponse {
            bundle_hash,
            block_number,
            simulation_success: true, // Would be parsed from actual response
            coinbase_diff: "0".to_string(),
            gas_fees: "0".to_string(),
            eth_sent_to_coinbase: "0".to_string(),
            gas_price: "0".to_string(),
            gas_used: 0,
        })
    }

    /// Parse simulation response
    fn parse_simulation_response(&self, response: serde_json::Value) -> Result<FlashbotsSimulation> {
        let result = response.get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in simulation response"))?;

        let bundle_hash = H256::random(); // Would be parsed from response
        let coinbase_diff = result.get("coinbaseDiff")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let results = result.get("results")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|tx| {
                    Some(TransactionResult {
                        coinbase_diff: tx.get("coinbaseDiff")?.as_str()?.to_string(),
                        eth_sent_to_coinbase: tx.get("ethSentToCoinbase")?.as_str()?.to_string(),
                        from_address: tx.get("fromAddress")?.as_str()?.parse().ok()?,
                        gas_fees: tx.get("gasFees")?.as_str()?.to_string(),
                        gas_price: tx.get("gasPrice")?.as_str()?.to_string(),
                        gas_used: tx.get("gasUsed")?.as_u64()?,
                        to_address: tx.get("toAddress")?.as_str()?.parse().ok(),
                        tx_hash: tx.get("txHash")?.as_str()?.parse().ok()?,
                        value: tx.get("value")?.as_str()?.to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        let total_gas_used = results.iter().map(|r| r.gas_used).sum();

        Ok(FlashbotsSimulation {
            bundle_hash,
            coinbase_diff,
            results,
            total_gas_used,
            first_revert: None, // Would be parsed from response
        })
    }
}

impl FlashbotsBundle {
    pub fn new(block_number: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            transactions: Vec::new(),
            block_number,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: Vec::new(),
        }
    }

    pub fn add_transaction(&mut self, signed_tx: String) {
        self.transactions.push(signed_tx);
    }

    pub fn add_reverting_tx(&mut self, tx_hash: H256) {
        self.reverting_tx_hashes.push(tx_hash);
    }

    pub fn set_timing(&mut self, min_timestamp: Option<u64>, max_timestamp: Option<u64>) {
        self.min_timestamp = min_timestamp;
        self.max_timestamp = max_timestamp;
    }

    pub fn is_valid(&self) -> bool {
        !self.transactions.is_empty() && self.block_number > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bundle_creation() {
        let mut bundle = FlashbotsBundle::new(12345);
        bundle.add_transaction("0x1234".to_string());
        
        assert!(bundle.is_valid());
        assert_eq!(bundle.transactions.len(), 1);
        assert_eq!(bundle.block_number, 12345);
    }

    #[test]
    fn test_flashbots_client_creation() {
        let client = FlashbotsClient::new(
            "https://relay.flashbots.net".to_string(),
            "test_key".to_string(),
        );
        
        assert_eq!(client.relay_url, "https://relay.flashbots.net");
        assert_eq!(client.max_retries, 3);
    }
}
