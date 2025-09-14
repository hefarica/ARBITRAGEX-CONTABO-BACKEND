use anyhow::Result;
use chrono::Utc;
use std::env;
use tokio::time::{Duration, interval};
use tracing::{info, error, warn};
use tracing_subscriber;

mod pnl;
mod sync;

use pnl::{PnLCalculator, ProfitComparator};
use sync::{SyncManager, SyncConfig, ConflictResolution};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();
    
    info!("🚀 Starting ArbitrageX Reconciliation Service v1.0");

    // Load configuration from environment
    let config = load_config()?;
    
    // Initialize database connection
    let postgres_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.postgres_url)
        .await?;

    // Run database migrations
    sqlx::migrate!("./migrations").run(&postgres_pool).await?;

    // Initialize P&L calculator
    let pnl_calculator = PnLCalculator::new(
        postgres_pool.clone(),
        &config.redis_url,
    )?;

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
