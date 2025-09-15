// ArbitrageX Supreme V3.0 - Shared Database Module
// Database connection pool and common queries

use crate::error::{ArbitrageError, ArbitrageResult};
use crate::types::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Database connection pool manager
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(database_url: &str, max_connections: u32) -> ArbitrageResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .map_err(|e| ArbitrageError::Database {
                message: format!("Failed to connect to database: {}", e),
            })?;

        Ok(Self { pool })
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run database migrations
    pub async fn migrate(&self) -> ArbitrageResult<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| ArbitrageError::Database {
                message: format!("Migration failed: {}", e),
            })?;

        Ok(())
    }

    /// Check database health
    pub async fn health_check(&self) -> ArbitrageResult<bool> {
        let result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("Database health check failed: {}", e);
                Ok(false)
            }
        }
    }

    // Opportunity CRUD operations
    pub async fn create_opportunity(&self, opportunity: &Opportunity) -> ArbitrageResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO opportunities (
                id, chain, token_a, token_b, dex_a, dex_b,
                amount_in, amount_out, profit_usd, gas_estimate, gas_price_gwei,
                price_impact, confidence_score, status, block_number,
                transaction_hash, created_at, updated_at, expires_at, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            "#,
            opportunity.id,
            opportunity.chain.to_string(),
            opportunity.token_a,
            opportunity.token_b,
            opportunity.dex_a,
            opportunity.dex_b,
            opportunity.amount_in,
            opportunity.amount_out,
            opportunity.profit_usd,
            opportunity.gas_estimate as i64,
            opportunity.gas_price_gwei as i64,
            opportunity.price_impact,
            opportunity.confidence_score,
            opportunity.status.to_string(),
            opportunity.block_number.map(|n| n as i64),
            opportunity.transaction_hash,
            opportunity.created_at,
            opportunity.updated_at,
            opportunity.expires_at,
            serde_json::to_value(&opportunity.metadata).unwrap_or(Value::Null)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to create opportunity: {}", e),
        })?;

        Ok(())
    }

    pub async fn get_opportunity(&self, id: Uuid) -> ArbitrageResult<Option<Opportunity>> {
        let row = sqlx::query!(
            "SELECT * FROM opportunities WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to get opportunity: {}", e),
        })?;

        match row {
            Some(row) => {
                let metadata: HashMap<String, Value> = serde_json::from_value(row.metadata.unwrap_or(Value::Null))
                    .unwrap_or_default();

                Ok(Some(Opportunity {
                    id: row.id,
                    chain: row.chain.into(),
                    token_a: row.token_a,
                    token_b: row.token_b,
                    dex_a: row.dex_a,
                    dex_b: row.dex_b,
                    amount_in: row.amount_in,
                    amount_out: row.amount_out,
                    profit_usd: row.profit_usd,
                    gas_estimate: row.gas_estimate as u64,
                    gas_price_gwei: row.gas_price_gwei as u64,
                    price_impact: row.price_impact,
                    confidence_score: row.confidence_score,
                    status: row.status.into(),
                    block_number: row.block_number.map(|n| n as u64),
                    transaction_hash: row.transaction_hash,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    expires_at: row.expires_at,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_opportunities(&self, query: &OpportunityQuery) -> ArbitrageResult<Vec<Opportunity>> {
        let mut sql = "SELECT * FROM opportunities WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn sqlx::Encode<sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 0;

        if let Some(chain) = &query.chain {
            param_count += 1;
            sql.push_str(&format!(" AND chain = ${}", param_count));
            params.push(Box::new(chain.to_string()));
        }

        if let Some(status) = &query.status {
            param_count += 1;
            sql.push_str(&format!(" AND status = ${}", param_count));
            params.push(Box::new(status.to_string()));
        }

        if let Some(min_profit) = query.min_profit {
            param_count += 1;
            sql.push_str(&format!(" AND profit_usd >= ${}", param_count));
            params.push(Box::new(min_profit));
        }

        if let Some(max_profit) = query.max_profit {
            param_count += 1;
            sql.push_str(&format!(" AND profit_usd <= ${}", param_count));
            params.push(Box::new(max_profit));
        }

        // Add sorting
        if let Some(sort_by) = &query.sort_by {
            let order = match query.sort_order {
                Some(SortOrder::Desc) => "DESC",
                _ => "ASC",
            };
            sql.push_str(&format!(" ORDER BY {} {}", sort_by, order));
        } else {
            sql.push_str(" ORDER BY created_at DESC");
        }

        // Add pagination
        let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE);
        let offset = query.offset.unwrap_or(0);
        
        param_count += 1;
        sql.push_str(&format!(" LIMIT ${}", param_count));
        params.push(Box::new(limit as i64));
        
        param_count += 1;
        sql.push_str(&format!(" OFFSET ${}", param_count));
        params.push(Box::new(offset as i64));

        // This is a simplified version - in practice you'd use a query builder
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ArbitrageError::Database {
                message: format!("Failed to list opportunities: {}", e),
            })?;

        let mut opportunities = Vec::new();
        for row in rows {
            let metadata: HashMap<String, Value> = serde_json::from_value(
                row.try_get::<Value, _>("metadata").unwrap_or(Value::Null)
            ).unwrap_or_default();

            opportunities.push(Opportunity {
                id: row.try_get("id")?,
                chain: row.try_get::<String, _>("chain")?.into(),
                token_a: row.try_get("token_a")?,
                token_b: row.try_get("token_b")?,
                dex_a: row.try_get("dex_a")?,
                dex_b: row.try_get("dex_b")?,
                amount_in: row.try_get("amount_in")?,
                amount_out: row.try_get("amount_out")?,
                profit_usd: row.try_get("profit_usd")?,
                gas_estimate: row.try_get::<i64, _>("gas_estimate")? as u64,
                gas_price_gwei: row.try_get::<i64, _>("gas_price_gwei")? as u64,
                price_impact: row.try_get("price_impact")?,
                confidence_score: row.try_get("confidence_score")?,
                status: row.try_get::<String, _>("status")?.into(),
                block_number: row.try_get::<Option<i64>, _>("block_number")?.map(|n| n as u64),
                transaction_hash: row.try_get("transaction_hash")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                expires_at: row.try_get("expires_at")?,
                metadata,
            });
        }

        Ok(opportunities)
    }

    pub async fn update_opportunity_status(&self, id: Uuid, status: OpportunityStatus) -> ArbitrageResult<()> {
        sqlx::query!(
            "UPDATE opportunities SET status = $1, updated_at = $2 WHERE id = $3",
            status.to_string(),
            Utc::now(),
            id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to update opportunity status: {}", e),
        })?;

        Ok(())
    }

    // Execution CRUD operations
    pub async fn create_execution(&self, execution: &Execution) -> ArbitrageResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO executions (
                id, opportunity_id, tx_hash, status, profit_expected_usd, profit_actual_usd,
                gas_estimate, gas_used, gas_price_estimate_gwei, gas_price_actual_gwei,
                block_number, executed_at, confirmed_at, error_message, retry_count, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
            execution.id,
            execution.opportunity_id,
            execution.tx_hash,
            execution.status.to_string(),
            execution.profit_expected_usd,
            execution.profit_actual_usd,
            execution.gas_estimate.map(|g| g as i64),
            execution.gas_used.map(|g| g as i64),
            execution.gas_price_estimate_gwei.map(|g| g as i64),
            execution.gas_price_actual_gwei.map(|g| g as i64),
            execution.block_number.map(|n| n as i64),
            execution.executed_at,
            execution.confirmed_at,
            execution.error_message,
            execution.retry_count as i32,
            serde_json::to_value(&execution.metadata).unwrap_or(Value::Null)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to create execution: {}", e),
        })?;

        Ok(())
    }

    pub async fn get_execution(&self, id: Uuid) -> ArbitrageResult<Option<Execution>> {
        let row = sqlx::query!(
            "SELECT * FROM executions WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to get execution: {}", e),
        })?;

        match row {
            Some(row) => {
                let metadata: HashMap<String, Value> = serde_json::from_value(row.metadata.unwrap_or(Value::Null))
                    .unwrap_or_default();

                Ok(Some(Execution {
                    id: row.id,
                    opportunity_id: row.opportunity_id,
                    tx_hash: row.tx_hash,
                    status: row.status.into(),
                    profit_expected_usd: row.profit_expected_usd,
                    profit_actual_usd: row.profit_actual_usd,
                    gas_estimate: row.gas_estimate.map(|g| g as u64),
                    gas_used: row.gas_used.map(|g| g as u64),
                    gas_price_estimate_gwei: row.gas_price_estimate_gwei.map(|g| g as u64),
                    gas_price_actual_gwei: row.gas_price_actual_gwei.map(|g| g as u64),
                    block_number: row.block_number.map(|n| n as u64),
                    executed_at: row.executed_at,
                    confirmed_at: row.confirmed_at,
                    error_message: row.error_message,
                    retry_count: row.retry_count as u32,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_executions(&self, query: &ExecutionQuery) -> ArbitrageResult<Vec<Execution>> {
        let mut sql = "SELECT * FROM executions WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn sqlx::Encode<sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 0;

        if let Some(opportunity_id) = query.opportunity_id {
            param_count += 1;
            sql.push_str(&format!(" AND opportunity_id = ${}", param_count));
            params.push(Box::new(opportunity_id));
        }

        if let Some(status) = &query.status {
            param_count += 1;
            sql.push_str(&format!(" AND status = ${}", param_count));
            params.push(Box::new(status.to_string()));
        }

        // Add sorting
        if let Some(sort_by) = &query.sort_by {
            let order = match query.sort_order {
                Some(SortOrder::Desc) => "DESC",
                _ => "ASC",
            };
            sql.push_str(&format!(" ORDER BY {} {}", sort_by, order));
        } else {
            sql.push_str(" ORDER BY executed_at DESC");
        }

        // Add pagination
        let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE);
        let offset = query.offset.unwrap_or(0);
        
        param_count += 1;
        sql.push_str(&format!(" LIMIT ${}", param_count));
        params.push(Box::new(limit as i64));
        
        param_count += 1;
        sql.push_str(&format!(" OFFSET ${}", param_count));
        params.push(Box::new(offset as i64));

        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ArbitrageError::Database {
                message: format!("Failed to list executions: {}", e),
            })?;

        let mut executions = Vec::new();
        for row in rows {
            let metadata: HashMap<String, Value> = serde_json::from_value(
                row.try_get::<Value, _>("metadata").unwrap_or(Value::Null)
            ).unwrap_or_default();

            executions.push(Execution {
                id: row.try_get("id")?,
                opportunity_id: row.try_get("opportunity_id")?,
                tx_hash: row.try_get("tx_hash")?,
                status: row.try_get::<String, _>("status")?.into(),
                profit_expected_usd: row.try_get("profit_expected_usd")?,
                profit_actual_usd: row.try_get("profit_actual_usd")?,
                gas_estimate: row.try_get::<Option<i64>, _>("gas_estimate")?.map(|g| g as u64),
                gas_used: row.try_get::<Option<i64>, _>("gas_used")?.map(|g| g as u64),
                gas_price_estimate_gwei: row.try_get::<Option<i64>, _>("gas_price_estimate_gwei")?.map(|g| g as u64),
                gas_price_actual_gwei: row.try_get::<Option<i64>, _>("gas_price_actual_gwei")?.map(|g| g as u64),
                block_number: row.try_get::<Option<i64>, _>("block_number")?.map(|n| n as u64),
                executed_at: row.try_get("executed_at")?,
                confirmed_at: row.try_get("confirmed_at")?,
                error_message: row.try_get("error_message")?,
                retry_count: row.try_get::<i32, _>("retry_count")? as u32,
                metadata,
            });
        }

        Ok(executions)
    }

    pub async fn update_execution_status(&self, id: Uuid, status: ExecutionStatus) -> ArbitrageResult<()> {
        sqlx::query!(
            "UPDATE executions SET status = $1 WHERE id = $2",
            status.to_string(),
            id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to update execution status: {}", e),
        })?;

        Ok(())
    }

    // System metrics
    pub async fn get_system_metrics(&self) -> ArbitrageResult<SystemMetrics> {
        let row = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_opportunities,
                (SELECT COUNT(*) FROM executions) as total_executions,
                (SELECT COUNT(*) FROM executions WHERE status = 'confirmed') as successful_executions,
                COALESCE(SUM(profit_usd), 0) as total_profit_usd,
                (SELECT COALESCE(SUM(gas_used * gas_price_actual_gwei / 1000000000.0 * 2000), 0) FROM executions WHERE gas_used IS NOT NULL) as total_gas_cost_usd,
                MAX(created_at) as last_opportunity
            FROM opportunities
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to get system metrics: {}", e),
        })?;

        let last_execution = sqlx::query!(
            "SELECT MAX(executed_at) as last_execution FROM executions"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to get last execution: {}", e),
        })?;

        let total_opportunities = row.total_opportunities.unwrap_or(0) as u64;
        let total_executions = row.total_executions.unwrap_or(0) as u64;
        let successful_executions = row.successful_executions.unwrap_or(0) as u64;
        let total_profit_usd = row.total_profit_usd.unwrap_or(Decimal::ZERO);
        let total_gas_cost_usd = Decimal::from_f64(row.total_gas_cost_usd.unwrap_or(0.0)).unwrap_or(Decimal::ZERO);
        let net_profit_usd = total_profit_usd - total_gas_cost_usd;
        let success_rate = if total_executions > 0 {
            successful_executions as f64 / total_executions as f64
        } else {
            0.0
        };
        let avg_profit_per_execution = if successful_executions > 0 {
            total_profit_usd / Decimal::from(successful_executions)
        } else {
            Decimal::ZERO
        };

        Ok(SystemMetrics {
            total_opportunities,
            total_executions,
            successful_executions,
            total_profit_usd,
            total_gas_cost_usd,
            net_profit_usd,
            success_rate,
            avg_profit_per_execution,
            uptime_seconds: 0, // This would be calculated elsewhere
            last_opportunity: row.last_opportunity,
            last_execution: last_execution.last_execution,
        })
    }

    // Cleanup expired opportunities
    pub async fn cleanup_expired_opportunities(&self) -> ArbitrageResult<u64> {
        let result = sqlx::query!(
            "DELETE FROM opportunities WHERE expires_at IS NOT NULL AND expires_at < NOW()"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArbitrageError::Database {
            message: format!("Failed to cleanup expired opportunities: {}", e),
        })?;

        Ok(result.rows_affected())
    }

    // Transaction wrapper
    pub async fn transaction<F, R>(&self, f: F) -> ArbitrageResult<R>
    where
        F: for<'c> FnOnce(&'c mut sqlx::Transaction<'_, sqlx::Postgres>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ArbitrageResult<R>> + Send + 'c>>,
    {
        let mut tx = self.pool.begin().await.map_err(|e| ArbitrageError::Database {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        let result = f(&mut tx).await;

        match result {
            Ok(value) => {
                tx.commit().await.map_err(|e| ArbitrageError::Database {
                    message: format!("Failed to commit transaction: {}", e),
                })?;
                Ok(value)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }
}

// Implement string conversions for enum types
impl ToString for OpportunityStatus {
    fn to_string(&self) -> String {
        match self {
            OpportunityStatus::Pending => "pending".to_string(),
            OpportunityStatus::Analyzing => "analyzing".to_string(),
            OpportunityStatus::Selected => "selected".to_string(),
            OpportunityStatus::Executing => "executing".to_string(),
            OpportunityStatus::Completed => "completed".to_string(),
            OpportunityStatus::Failed => "failed".to_string(),
            OpportunityStatus::Expired => "expired".to_string(),
        }
    }
}

impl ToString for ExecutionStatus {
    fn to_string(&self) -> String {
        match self {
            ExecutionStatus::Pending => "pending".to_string(),
            ExecutionStatus::Submitted => "submitted".to_string(),
            ExecutionStatus::Confirmed => "confirmed".to_string(),
            ExecutionStatus::Failed => "failed".to_string(),
            ExecutionStatus::Reverted => "reverted".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection() {
        // This would require a test database
        // let db = Database::new("postgresql://test:test@localhost/test", 5).await;
        // assert!(db.is_ok());
    }

    #[test]
    fn test_enum_conversions() {
        assert_eq!(OpportunityStatus::Pending.to_string(), "pending");
        assert_eq!(ExecutionStatus::Confirmed.to_string(), "confirmed");
        
        let status: OpportunityStatus = "pending".to_string().into();
        assert_eq!(status, OpportunityStatus::Pending);
    }
}
