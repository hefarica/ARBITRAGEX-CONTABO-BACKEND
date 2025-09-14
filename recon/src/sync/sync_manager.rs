use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{Duration, interval};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub sync_id: String,
    pub source: String,
    pub destination: String,
    pub last_sync_time: DateTime<Utc>,
    pub records_synced: u64,
    pub sync_duration_ms: u64,
    pub status: SyncStatusType,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatusType {
    Success,
    Failed,
    InProgress,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub source_db_url: String,
    pub destination_db_url: String,
    pub sync_interval_minutes: u64,
    pub batch_size: u32,
    pub tables_to_sync: Vec<String>,
    pub conflict_resolution: ConflictResolution,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    SourceWins,
    DestinationWins,
    Timestamp,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub table_name: String,
    pub record_id: String,
    pub operation: SyncOperation,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncOperation {
    Insert,
    Update,
    Delete,
}

pub struct SyncManager {
    postgres_pool: sqlx::PgPool,
    d1_client: Option<D1Client>, // Would be actual D1 client
    sync_config: SyncConfig,
    sync_history: Vec<SyncStatus>,
}

impl SyncManager {
    pub fn new(postgres_pool: sqlx::PgPool, sync_config: SyncConfig) -> Self {
        Self {
            postgres_pool,
            d1_client: None, // Would initialize D1 client
            sync_config,
            sync_history: Vec::new(),
        }
    }

    /// Start the sync manager with periodic synchronization
    pub async fn start(&mut self) -> Result<()> {
        info!("🔄 Starting sync manager with {} minute intervals", self.sync_config.sync_interval_minutes);

        if !self.sync_config.enabled {
            warn!("⚠️ Sync manager is disabled in configuration");
            return Ok(());
        }

        let mut sync_interval = interval(Duration::from_secs(self.sync_config.sync_interval_minutes * 60));

        loop {
            sync_interval.tick().await;
            
            match self.perform_full_sync().await {
                Ok(status) => {
                    info!("✅ Sync completed successfully: {} records synced", status.records_synced);
                    self.sync_history.push(status);
                },
                Err(e) => {
                    error!("❌ Sync failed: {}", e);
                    self.sync_history.push(SyncStatus {
                        sync_id: Uuid::new_v4().to_string(),
                        source: "PostgreSQL".to_string(),
                        destination: "Cloudflare D1".to_string(),
                        last_sync_time: Utc::now(),
                        records_synced: 0,
                        sync_duration_ms: 0,
                        status: SyncStatusType::Failed,
                        error_message: Some(e.to_string()),
                    });
                }
            }

            // Keep only last 100 sync records
            if self.sync_history.len() > 100 {
                self.sync_history.drain(0..self.sync_history.len() - 100);
            }
        }
    }

    /// Perform a full synchronization between PostgreSQL and D1
    pub async fn perform_full_sync(&self) -> Result<SyncStatus> {
        let sync_id = Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        
        info!("🔄 Starting full sync: {}", sync_id);

        let mut total_records_synced = 0;

        for table_name in &self.sync_config.tables_to_sync {
            match self.sync_table(table_name).await {
                Ok(count) => {
                    total_records_synced += count;
                    debug!("✅ Synced {} records from table {}", count, table_name);
                },
                Err(e) => {
                    error!("❌ Failed to sync table {}: {}", table_name, e);
                    return Err(e);
                }
            }
        }

        let sync_duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(SyncStatus {
            sync_id,
            source: "PostgreSQL".to_string(),
            destination: "Cloudflare D1".to_string(),
            last_sync_time: Utc::now(),
            records_synced: total_records_synced,
            sync_duration_ms,
            status: SyncStatusType::Success,
            error_message: None,
        })
    }

    /// Sync a specific table
    async fn sync_table(&self, table_name: &str) -> Result<u64> {
        debug!("🔄 Syncing table: {}", table_name);

        match table_name {
            "opportunities" => self.sync_opportunities().await,
            "executions" => self.sync_executions().await,
            "pnl_calculations" => self.sync_pnl_calculations().await,
            "profit_comparisons" => self.sync_profit_comparisons().await,
            "strategies" => self.sync_strategies().await,
            _ => {
                warn!("⚠️ Unknown table for sync: {}", table_name);
                Ok(0)
            }
        }
    }

    /// Sync opportunities table
    async fn sync_opportunities(&self) -> Result<u64> {
        let last_sync_time = self.get_last_sync_time("opportunities").await?;
        
        let rows = sqlx::query!(
            "SELECT id, chain_id, strategy_type, tokens, pools, expected_profit, 
                    gas_cost, confidence, deadline, created_at, metadata
             FROM opportunities 
             WHERE created_at > $1 
             ORDER BY created_at ASC 
             LIMIT $2",
            last_sync_time,
            self.sync_config.batch_size as i64
        )
        .fetch_all(&self.postgres_pool)
        .await?;

        let mut synced_count = 0;

        for row in rows {
            let sync_record = SyncRecord {
                table_name: "opportunities".to_string(),
                record_id: row.id.to_string(),
                operation: SyncOperation::Insert, // Would determine actual operation
                data: serde_json::json!({
                    "id": row.id,
                    "chain_id": row.chain_id,
                    "strategy_type": row.strategy_type,
                    "tokens": row.tokens,
                    "pools": row.pools,
                    "expected_profit": row.expected_profit,
                    "gas_cost": row.gas_cost,
                    "confidence": row.confidence,
                    "deadline": row.deadline,
                    "created_at": row.created_at,
                    "metadata": row.metadata
                }),
                timestamp: row.created_at,
                checksum: self.calculate_checksum(&row.id.to_string())?,
            };

            self.sync_record_to_d1(&sync_record).await?;
            synced_count += 1;
        }

        if synced_count > 0 {
            self.update_last_sync_time("opportunities").await?;
        }

        Ok(synced_count)
    }

    /// Sync executions table
    async fn sync_executions(&self) -> Result<u64> {
        let last_sync_time = self.get_last_sync_time("executions").await?;
        
        let rows = sqlx::query!(
            "SELECT id, opportunity_id, tx_hash, status, actual_profit, 
                    gas_used, error_reason, created_at
             FROM executions 
             WHERE created_at > $1 
             ORDER BY created_at ASC 
             LIMIT $2",
            last_sync_time,
            self.sync_config.batch_size as i64
        )
        .fetch_all(&self.postgres_pool)
        .await?;

        let mut synced_count = 0;

        for row in rows {
            let sync_record = SyncRecord {
                table_name: "executions".to_string(),
                record_id: row.id.to_string(),
                operation: SyncOperation::Insert,
                data: serde_json::json!({
                    "id": row.id,
                    "opportunity_id": row.opportunity_id,
                    "tx_hash": row.tx_hash,
                    "status": row.status,
                    "actual_profit": row.actual_profit,
                    "gas_used": row.gas_used,
                    "error_reason": row.error_reason,
                    "created_at": row.created_at
                }),
                timestamp: row.created_at,
                checksum: self.calculate_checksum(&row.id.to_string())?,
            };

            self.sync_record_to_d1(&sync_record).await?;
            synced_count += 1;
        }

        if synced_count > 0 {
            self.update_last_sync_time("executions").await?;
        }

        Ok(synced_count)
    }

    /// Sync P&L calculations table
    async fn sync_pnl_calculations(&self) -> Result<u64> {
        let last_sync_time = self.get_last_sync_time("pnl_calculations").await?;
        
        let rows = sqlx::query!(
            "SELECT execution_id, opportunity_id, strategy_type, expected_profit,
                    actual_profit, gas_cost, net_profit, profit_variance,
                    execution_time, tokens_involved, dexes_used
             FROM pnl_calculations 
             WHERE execution_time > $1 
             ORDER BY execution_time ASC 
             LIMIT $2",
            last_sync_time,
            self.sync_config.batch_size as i64
        )
        .fetch_all(&self.postgres_pool)
        .await?;

        let mut synced_count = 0;

        for row in rows {
            let sync_record = SyncRecord {
                table_name: "pnl_calculations".to_string(),
                record_id: row.execution_id.clone(),
                operation: SyncOperation::Insert,
                data: serde_json::json!({
                    "execution_id": row.execution_id,
                    "opportunity_id": row.opportunity_id,
                    "strategy_type": row.strategy_type,
                    "expected_profit": row.expected_profit,
                    "actual_profit": row.actual_profit,
                    "gas_cost": row.gas_cost,
                    "net_profit": row.net_profit,
                    "profit_variance": row.profit_variance,
                    "execution_time": row.execution_time,
                    "tokens_involved": row.tokens_involved,
                    "dexes_used": row.dexes_used
                }),
                timestamp: row.execution_time,
                checksum: self.calculate_checksum(&row.execution_id)?,
            };

            self.sync_record_to_d1(&sync_record).await?;
            synced_count += 1;
        }

        if synced_count > 0 {
            self.update_last_sync_time("pnl_calculations").await?;
        }

        Ok(synced_count)
    }

    /// Sync profit comparisons table
    async fn sync_profit_comparisons(&self) -> Result<u64> {
        // Similar implementation to other sync methods
        Ok(0) // Placeholder
    }

    /// Sync strategies table
    async fn sync_strategies(&self) -> Result<u64> {
        // Similar implementation to other sync methods
        Ok(0) // Placeholder
    }

    /// Sync a record to Cloudflare D1
    async fn sync_record_to_d1(&self, record: &SyncRecord) -> Result<()> {
        // In a real implementation, this would use the Cloudflare D1 API
        // to insert/update/delete records in the edge database
        debug!("📤 Syncing record {} to D1: {}", record.record_id, record.table_name);
        
        // Placeholder for D1 sync logic
        tokio::time::sleep(Duration::from_millis(10)).await; // Simulate API call
        
        Ok(())
    }

    /// Get the last sync time for a table
    async fn get_last_sync_time(&self, table_name: &str) -> Result<DateTime<Utc>> {
        let row = sqlx::query!(
            "SELECT last_sync_time FROM sync_metadata WHERE table_name = $1",
            table_name
        )
        .fetch_optional(&self.postgres_pool)
        .await?;

        Ok(row.map(|r| r.last_sync_time).unwrap_or_else(|| {
            Utc::now() - chrono::Duration::days(30) // Default to 30 days ago
        }))
    }

    /// Update the last sync time for a table
    async fn update_last_sync_time(&self, table_name: &str) -> Result<()> {
        sqlx::query!(
            "INSERT INTO sync_metadata (table_name, last_sync_time) 
             VALUES ($1, $2) 
             ON CONFLICT (table_name) 
             DO UPDATE SET last_sync_time = $2",
            table_name,
            Utc::now()
        )
        .execute(&self.postgres_pool)
        .await?;

        Ok(())
    }

    /// Calculate checksum for data integrity
    fn calculate_checksum(&self, data: &str) -> Result<String> {
        use sha3::{Digest, Sha3_256};
        
        let mut hasher = Sha3_256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        
        Ok(hex::encode(result))
    }

    /// Get sync statistics
    pub fn get_sync_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        let successful_syncs = self.sync_history.iter()
            .filter(|s| matches!(s.status, SyncStatusType::Success))
            .count();

        let failed_syncs = self.sync_history.iter()
            .filter(|s| matches!(s.status, SyncStatusType::Failed))
            .count();

        let total_records_synced: u64 = self.sync_history.iter()
            .filter(|s| matches!(s.status, SyncStatusType::Success))
            .map(|s| s.records_synced)
            .sum();

        let average_sync_duration: u64 = if successful_syncs > 0 {
            self.sync_history.iter()
                .filter(|s| matches!(s.status, SyncStatusType::Success))
                .map(|s| s.sync_duration_ms)
                .sum::<u64>() / successful_syncs as u64
        } else {
            0
        };

        stats.insert("successful_syncs".to_string(), serde_json::json!(successful_syncs));
        stats.insert("failed_syncs".to_string(), serde_json::json!(failed_syncs));
        stats.insert("total_records_synced".to_string(), serde_json::json!(total_records_synced));
        stats.insert("average_sync_duration_ms".to_string(), serde_json::json!(average_sync_duration));
        stats.insert("last_sync_time".to_string(), 
                    serde_json::json!(self.sync_history.last().map(|s| s.last_sync_time)));

        stats
    }

    /// Perform manual sync for a specific table
    pub async fn manual_sync_table(&self, table_name: &str) -> Result<SyncStatus> {
        let sync_id = Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        
        info!("🔄 Starting manual sync for table: {}", table_name);

        match self.sync_table(table_name).await {
            Ok(records_synced) => {
                let sync_duration_ms = start_time.elapsed().as_millis() as u64;
                
                Ok(SyncStatus {
                    sync_id,
                    source: "PostgreSQL".to_string(),
                    destination: "Cloudflare D1".to_string(),
                    last_sync_time: Utc::now(),
                    records_synced,
                    sync_duration_ms,
                    status: SyncStatusType::Success,
                    error_message: None,
                })
            },
            Err(e) => {
                Ok(SyncStatus {
                    sync_id,
                    source: "PostgreSQL".to_string(),
                    destination: "Cloudflare D1".to_string(),
                    last_sync_time: Utc::now(),
                    records_synced: 0,
                    sync_duration_ms: start_time.elapsed().as_millis() as u64,
                    status: SyncStatusType::Failed,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }
}

// Placeholder for D1 client
struct D1Client;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_creation() {
        let status = SyncStatus {
            sync_id: "test".to_string(),
            source: "PostgreSQL".to_string(),
            destination: "D1".to_string(),
            last_sync_time: Utc::now(),
            records_synced: 100,
            sync_duration_ms: 5000,
            status: SyncStatusType::Success,
            error_message: None,
        };

        assert_eq!(status.records_synced, 100);
        assert!(matches!(status.status, SyncStatusType::Success));
    }

    #[test]
    fn test_sync_config_creation() {
        let config = SyncConfig {
            source_db_url: "postgresql://test".to_string(),
            destination_db_url: "d1://test".to_string(),
            sync_interval_minutes: 30,
            batch_size: 1000,
            tables_to_sync: vec!["opportunities".to_string(), "executions".to_string()],
            conflict_resolution: ConflictResolution::Timestamp,
            enabled: true,
        };

        assert!(config.enabled);
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.tables_to_sync.len(), 2);
    }
}
