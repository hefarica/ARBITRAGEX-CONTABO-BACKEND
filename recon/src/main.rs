// ArbitrageX Supreme V3.0 - Blockchain Reconciliation Service
// P&L analysis, database synchronization, and transaction verification

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use ethers::{
    core::types::{Address, H256, U256},
    providers::{Http, Middleware, Provider, Ws},
    types::{Block, Transaction, TransactionReceipt},
};
use redis::AsyncCommands;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::broadcast,
    time::{interval, sleep},
};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

// Core data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationRecord {
    pub id: String,
    pub execution_id: String,
    pub opportunity_id: String,
    pub chain: String,
    pub tx_hash: Option<String>,
    pub block_number: Option<u64>,
    pub status: ReconciliationStatus,
    pub expected_profit_usd: Decimal,
    pub actual_profit_usd: Option<Decimal>,
    pub gas_estimate: u64,
    pub gas_used: Option<u64>,
    pub gas_price_gwei: u64,
    pub discrepancies: Vec<Discrepancy>,
    pub created_at: DateTime<Utc>,
    pub reconciled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconciliationStatus {
    Pending,
    InProgress,
    Reconciled,
    Failed,
    Discrepancy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discrepancy {
    pub field: String,
    pub expected: String,
    pub actual: String,
    pub severity: DiscrepancySeverity,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscrepancySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub pending_reconciliations: usize,
    pub total_reconciled: u64,
    pub success_rate: f64,
    pub avg_reconciliation_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PnLSummary {
    pub total_profit_usd: Decimal,
    pub total_gas_cost_usd: Decimal,
    pub net_profit_usd: Decimal,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub success_rate: f64,
    pub avg_profit_per_execution: Decimal,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

// Application state
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub reconciliations: Arc<DashMap<String, ReconciliationRecord>>,
    pub metrics: Arc<RwLock<SystemMetrics>>,
    pub config: Arc<AppConfig>,
    pub event_sender: broadcast::Sender<ReconciliationEvent>,
    pub providers: Arc<DashMap<String, Arc<Provider<Ws>>>>,
    pub start_time: SystemTime,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub total_reconciliations: u64,
    pub successful_reconciliations: u64,
    pub failed_reconciliations: u64,
    pub avg_reconciliation_time_ms: f64,
    pub total_profit_usd: Decimal,
    pub total_gas_cost_usd: Decimal,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub port: u16,
    pub reconciliation_interval_seconds: u64,
    pub max_pending_reconciliations: usize,
    pub supported_chains: Vec<ChainConfig>,
    pub eth_price_usd: Decimal,
}

#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub confirmations_required: u64,
}

#[derive(Debug, Clone, Serialize)]
pub enum ReconciliationEvent {
    Started(String),
    Completed(String),
    Failed(String, String),
    DiscrepancyFound(String, Discrepancy),
}

// Core reconciliation logic
#[instrument(skip(state))]
async fn reconcile_execution(
    state: &AppState,
    execution_id: &str,
) -> Result<ReconciliationRecord> {
    // Get execution details from database
    let execution = get_execution_details(&state.db, execution_id).await?;
    
    let mut record = ReconciliationRecord {
        id: Uuid::new_v4().to_string(),
        execution_id: execution_id.to_string(),
        opportunity_id: execution.opportunity_id,
        chain: execution.chain.clone(),
        tx_hash: execution.tx_hash.clone(),
        block_number: None,
        status: ReconciliationStatus::InProgress,
        expected_profit_usd: execution.expected_profit_usd,
        actual_profit_usd: None,
        gas_estimate: execution.gas_estimate,
        gas_used: None,
        gas_price_gwei: execution.gas_price_gwei,
        discrepancies: vec![],
        created_at: Utc::now(),
        reconciled_at: None,
    };
    
    // If we have a transaction hash, verify on-chain
    if let Some(ref tx_hash) = execution.tx_hash {
        match verify_transaction_on_chain(state, &execution.chain, tx_hash).await {
            Ok(chain_data) => {
                record.block_number = Some(chain_data.block_number);
                record.gas_used = Some(chain_data.gas_used);
                record.actual_profit_usd = Some(chain_data.profit_usd);
                
                // Check for discrepancies
                record.discrepancies = find_discrepancies(&execution, &chain_data);
                
                if record.discrepancies.is_empty() {
                    record.status = ReconciliationStatus::Reconciled;
                } else {
                    record.status = ReconciliationStatus::Discrepancy;
                }
            }
            Err(e) => {
                warn!("Failed to verify transaction on-chain: {}", e);
                record.status = ReconciliationStatus::Failed;
                record.discrepancies.push(Discrepancy {
                    field: "transaction_verification".to_string(),
                    expected: "transaction found on-chain".to_string(),
                    actual: format!("verification failed: {}", e),
                    severity: DiscrepancySeverity::High,
                    description: "Could not verify transaction on blockchain".to_string(),
                });
            }
        }
    } else {
        // No transaction hash - execution likely failed
        record.status = ReconciliationStatus::Failed;
        record.discrepancies.push(Discrepancy {
            field: "transaction_hash".to_string(),
            expected: "valid transaction hash".to_string(),
            actual: "null".to_string(),
            severity: DiscrepancySeverity::Critical,
            description: "Execution has no transaction hash".to_string(),
        });
    }
    
    record.reconciled_at = Some(Utc::now());
    
    // Store reconciliation record
    store_reconciliation_record(&state.db, &record).await?;
    
    Ok(record)
}

#[derive(Debug)]
struct ExecutionDetails {
    opportunity_id: String,
    chain: String,
    tx_hash: Option<String>,
    expected_profit_usd: Decimal,
    gas_estimate: u64,
    gas_price_gwei: u64,
}

#[derive(Debug)]
struct ChainData {
    block_number: u64,
    gas_used: u64,
    profit_usd: Decimal,
}

async fn get_execution_details(
    db: &PgPool,
    execution_id: &str,
) -> Result<ExecutionDetails> {
    let row = sqlx::query!(
        "SELECT opportunity_id, tx_hash, profit_actual_usd, gas_used, gas_price_actual_gwei FROM executions WHERE id = $1",
        execution_id
    )
    .fetch_one(db)
    .await
    .context("Failed to fetch execution details")?;
    
    Ok(ExecutionDetails {
        opportunity_id: row.opportunity_id,
        chain: "ethereum".to_string(), // This should come from the opportunity
        tx_hash: row.tx_hash,
        expected_profit_usd: Decimal::from_f64(row.profit_actual_usd.unwrap_or(0.0)).unwrap_or_default(),
        gas_estimate: row.gas_used.unwrap_or(0) as u64,
        gas_price_gwei: row.gas_price_actual_gwei.unwrap_or(0) as u64,
    })
}

async fn verify_transaction_on_chain(
    state: &AppState,
    chain: &str,
    tx_hash: &str,
) -> Result<ChainData> {
    let provider = state.providers.get(chain)
        .ok_or_else(|| anyhow::anyhow!("No provider for chain: {}", chain))?;
    
    let tx_hash = H256::from_str(tx_hash)
        .context("Invalid transaction hash")?;
    
    // Get transaction receipt
    let receipt = provider.get_transaction_receipt(tx_hash)
        .await
        .context("Failed to get transaction receipt")?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    
    // Calculate profit from logs (simplified)
    let profit_usd = calculate_profit_from_logs(&receipt)?;
    
    Ok(ChainData {
        block_number: receipt.block_number.unwrap_or_default().as_u64(),
        gas_used: receipt.gas_used.unwrap_or_default().as_u64(),
        profit_usd,
    })
}

fn calculate_profit_from_logs(receipt: &TransactionReceipt) -> Result<Decimal> {
    // This is a simplified implementation
    // In reality, you would parse the logs to calculate actual profit
    Ok(Decimal::from(0))
}

fn find_discrepancies(
    execution: &ExecutionDetails,
    chain_data: &ChainData,
) -> Vec<Discrepancy> {
    let mut discrepancies = vec![];
    
    // Check gas usage discrepancy
    let gas_diff = (chain_data.gas_used as i64 - execution.gas_estimate as i64).abs();
    let gas_tolerance = execution.gas_estimate / 10; // 10% tolerance
    
    if gas_diff > gas_tolerance as i64 {
        discrepancies.push(Discrepancy {
            field: "gas_used".to_string(),
            expected: execution.gas_estimate.to_string(),
            actual: chain_data.gas_used.to_string(),
            severity: if gas_diff > (gas_tolerance * 2) as i64 {
                DiscrepancySeverity::High
            } else {
                DiscrepancySeverity::Medium
            },
            description: format!("Gas usage differs by {} units", gas_diff),
        });
    }
    
    // Check profit discrepancy
    let profit_diff = (chain_data.profit_usd - execution.expected_profit_usd).abs();
    let profit_tolerance = execution.expected_profit_usd * Decimal::from_f64(0.05).unwrap(); // 5% tolerance
    
    if profit_diff > profit_tolerance {
        discrepancies.push(Discrepancy {
            field: "profit_usd".to_string(),
            expected: execution.expected_profit_usd.to_string(),
            actual: chain_data.profit_usd.to_string(),
            severity: if profit_diff > profit_tolerance * Decimal::from(2) {
                DiscrepancySeverity::High
            } else {
                DiscrepancySeverity::Medium
            },
            description: format!("Profit differs by ${}", profit_diff),
        });
    }
    
    discrepancies
}

async fn store_reconciliation_record(
    db: &PgPool,
    record: &ReconciliationRecord,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO reconciliation_records (
            id, execution_id, opportunity_id, chain, tx_hash, block_number,
            status, expected_profit_usd, actual_profit_usd, gas_estimate,
            gas_used, gas_price_gwei, discrepancies, created_at, reconciled_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        "#,
        record.id,
        record.execution_id,
        record.opportunity_id,
        record.chain,
        record.tx_hash,
        record.block_number.map(|x| x as i64),
        format!("{:?}", record.status),
        record.expected_profit_usd.to_f64(),
        record.actual_profit_usd.map(|x| x.to_f64()),
        record.gas_estimate as i64,
        record.gas_used.map(|x| x as i64),
        record.gas_price_gwei as i64,
        serde_json::to_value(&record.discrepancies).unwrap(),
        record.created_at,
        record.reconciled_at
    )
    .execute(db)
    .await
    .context("Failed to store reconciliation record")?;
    
    Ok(())
}

// Background reconciliation task
async fn reconciliation_worker(state: AppState) {
    let mut interval = interval(Duration::from_secs(state.config.reconciliation_interval_seconds));
    
    loop {
        interval.tick().await;
        
        // Get pending executions
        match get_pending_executions(&state.db).await {
            Ok(executions) => {
                for execution_id in executions {
                    if state.reconciliations.len() >= state.config.max_pending_reconciliations {
                        warn!("Max pending reconciliations reached, skipping");
                        break;
                    }
                    
                    let worker_state = state.clone();
                    let exec_id = execution_id.clone();
                    
                    tokio::spawn(async move {
                        match reconcile_execution(&worker_state, &exec_id).await {
                            Ok(record) => {
                                worker_state.reconciliations.insert(record.id.clone(), record.clone());
                                let _ = worker_state.event_sender.send(ReconciliationEvent::Completed(record.id));
                            }
                            Err(e) => {
                                error!("Reconciliation failed for execution {}: {}", exec_id, e);
                                let _ = worker_state.event_sender.send(ReconciliationEvent::Failed(exec_id, e.to_string()));
                            }
                        }
                    });
                }
            }
            Err(e) => {
                error!("Failed to get pending executions: {}", e);
            }
        }
    }
}

async fn get_pending_executions(db: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query!(
        "SELECT id FROM executions WHERE status = 'confirmed' AND id NOT IN (SELECT execution_id FROM reconciliation_records) LIMIT 100"
    )
    .fetch_all(db)
    .await
    .context("Failed to fetch pending executions")?;
    
    Ok(rows.into_iter().map(|row| row.id).collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("recon=debug,info")
        .json()
        .init();
    
    info!("Starting ArbitrageX Supreme V3.0 Reconciliation Service");
    
    // Load configuration
    let config = Arc::new(AppConfig {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://arbitragex:password@localhost:5432/arbitragex_supreme".to_string()),
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        port: std::env::var("RECON_PORT")
            .unwrap_or_else(|_| "8083".to_string())
            .parse()
            .unwrap_or(8083),
        reconciliation_interval_seconds: 60,
        max_pending_reconciliations: 100,
        supported_chains: vec![
            ChainConfig {
                name: "ethereum".to_string(),
                chain_id: 1,
                rpc_url: std::env::var("ETHEREUM_RPC_URL").unwrap_or_default(),
                confirmations_required: 1,
            },
    // Initialize profit comparator
    let profit_comparator = ProfitComparator::new(postgres_pool.clone());

    // Initialize sync manager
    let sync_config = SyncConfig {
        source_db_url: config.postgres_url.clone(),
        destination_db_url: config.d1_url.clone(),
        sync_interval_minutes: config.sync_interval_minutes,
        batch_size: config.sync_batch_size,
        tables_to_sync: vec![
            "opportunities".to_string(),
            "executions".to_string(),
            "pnl_calculations".to_string(),
            "profit_comparisons".to_string(),
            "strategies".to_string(),
        ],
        conflict_resolution: ConflictResolution::Timestamp,
        enabled: config.sync_enabled,
    };

    let mut sync_manager = SyncManager::new(postgres_pool.clone(), sync_config);

    // Start background tasks
    info!("🔄 Starting background reconciliation tasks...");

    // Task 1: P&L calculation for new executions
    let pnl_calc_task = {
        let pnl_calculator = pnl_calculator.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // Every 5 minutes
            
            loop {
                interval.tick().await;
                
                match process_pending_pnl_calculations(&pnl_calculator).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("✅ Processed {} P&L calculations", count);
                        }
                    },
                    Err(e) => {
                        error!("❌ P&L calculation task failed: {}", e);
                    }
                }
            }
        })
    };

    // Task 2: Profit comparison analysis
    let profit_comp_task = {
        let profit_comparator = profit_comparator.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(600)); // Every 10 minutes
            
            loop {
                interval.tick().await;
                
                match process_profit_comparisons(&profit_comparator).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("✅ Processed {} profit comparisons", count);
                        }
                    },
                    Err(e) => {
                        error!("❌ Profit comparison task failed: {}", e);
                    }
                }
            }
        })
    };

    // Task 3: Database synchronization
    let sync_task = {
        tokio::spawn(async move {
            if let Err(e) = sync_manager.start().await {
                error!("❌ Sync manager failed: {}", e);
            }
        })
    };

    // Task 4: Generate periodic reports
    let reporting_task = {
        let pnl_calculator = pnl_calculator.clone();
        let profit_comparator = profit_comparator.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Every hour
            
            loop {
                interval.tick().await;
                
                match generate_periodic_reports(&pnl_calculator, &profit_comparator).await {
                    Ok(_) => {
                        info!("✅ Generated periodic reports");
                    },
                    Err(e) => {
                        error!("❌ Report generation failed: {}", e);
                    }
                }
            }
        })
    };

    // Task 5: Health monitoring
    let health_task = {
        let postgres_pool = postgres_pool.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Every minute
            
            loop {
                interval.tick().await;
                
                match health_check(&postgres_pool).await {
                    Ok(_) => {
                        // Health check passed silently
                    },
                    Err(e) => {
                        error!("❌ Health check failed: {}", e);
                    }
                }
            }
        })
    };

    info!("✅ All reconciliation tasks started successfully");

    // Wait for all tasks
    tokio::select! {
        _ = pnl_calc_task => warn!("P&L calculation task ended"),
        _ = profit_comp_task => warn!("Profit comparison task ended"),
        _ = sync_task => warn!("Sync task ended"),
        _ = reporting_task => warn!("Reporting task ended"),
        _ = health_task => warn!("Health task ended"),
    }

    Ok(())
}

async fn process_pending_pnl_calculations(calculator: &PnLCalculator) -> Result<u32> {
    // Get executions that don't have P&L calculations yet
    // This would query the database for unprocessed executions
    // For now, return 0 as placeholder
    Ok(0)
}

async fn process_profit_comparisons(comparator: &ProfitComparator) -> Result<u32> {
    // Get executions that need profit comparison analysis
    // This would query for executions with simulations but no comparisons
    // For now, return 0 as placeholder
    Ok(0)
}

async fn generate_periodic_reports(
    pnl_calculator: &PnLCalculator,
    profit_comparator: &ProfitComparator,
) -> Result<()> {
    let end_time = Utc::now();
    let start_time = end_time - chrono::Duration::hours(1);

    // Generate hourly P&L summary
    let pnl_summary = pnl_calculator.calculate_period_summary(
        start_time,
        end_time,
        None,
    ).await?;

    // Generate accuracy metrics
    let accuracy_metrics = profit_comparator.get_accuracy_metrics(
        start_time,
        end_time,
    ).await?;

    info!("📊 Hourly Report - Executions: {}, Net Profit: {}, Accuracy: {}%", 
          pnl_summary.total_executions,
          pnl_summary.total_net_profit,
          accuracy_metrics.average_profit_accuracy);

    Ok(())
}

async fn health_check(postgres_pool: &sqlx::PgPool) -> Result<()> {
    // Check database connectivity
    sqlx::query("SELECT 1")
        .fetch_one(postgres_pool)
        .await?;

    Ok(())
}

#[derive(Debug)]
struct ReconConfig {
    postgres_url: String,
    redis_url: String,
    d1_url: String,
    sync_enabled: bool,
    sync_interval_minutes: u64,
    sync_batch_size: u32,
}

fn load_config() -> Result<ReconConfig> {
    let postgres_url = env::var("DATABASE_URL")
        .or_else(|_| env::var("POSTGRES_URL"))
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/arbitragex".to_string());

    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let d1_url = env::var("D1_DATABASE_URL")
        .unwrap_or_else(|_| "d1://arbitragex-edge".to_string());

    let sync_enabled = env::var("SYNC_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .unwrap_or(true);

    let sync_interval_minutes = env::var("SYNC_INTERVAL_MINUTES")
        .unwrap_or_else(|_| "30".to_string())
        .parse()
        .unwrap_or(30);

    let sync_batch_size = env::var("SYNC_BATCH_SIZE")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or(1000);

    Ok(ReconConfig {
        postgres_url,
        redis_url,
        d1_url,
        sync_enabled,
        sync_interval_minutes,
        sync_batch_size,
    })
}
