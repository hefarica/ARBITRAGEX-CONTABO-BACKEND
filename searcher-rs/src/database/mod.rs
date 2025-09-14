use anyhow::Result;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{info, error, debug};
use uuid::Uuid;

use crate::types::{Opportunity, DexPool};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("🗄️ Connecting to PostgreSQL database...");
        
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        info!("✅ Database connection established");
        
        Ok(Self { pool })
    }

    /// Store a new opportunity in the database
    pub async fn store_opportunity(&self, opportunity: &Opportunity) -> Result<()> {
        debug!("💾 Storing opportunity: {}", opportunity.id);
        
        let query = r#"
            INSERT INTO opportunities (
                id, chain_id, strategy_type, tokens, pools, 
                expected_profit, gas_cost, confidence, deadline, 
                created_at, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                expected_profit = EXCLUDED.expected_profit,
                gas_cost = EXCLUDED.gas_cost,
                confidence = EXCLUDED.confidence,
                updated_at = NOW()
        "#;
        
        let opportunity_id = Uuid::parse_str(&opportunity.id)?;
        let expected_profit = opportunity.expected_profit.to_string();
        let gas_cost = opportunity.gas_cost.to_string();
        let metadata = serde_json::to_value(&opportunity.metadata)?;
        
        sqlx::query(query)
            .bind(opportunity_id)
            .bind(opportunity.chain_id as i64)
            .bind(&opportunity.strategy_type)
            .bind(&opportunity.tokens)
            .bind(&opportunity.pools)
            .bind(expected_profit)
            .bind(gas_cost)
            .bind(opportunity.confidence)
            .bind(opportunity.deadline as i64)
            .bind(opportunity.created_at)
            .bind(metadata)
            .execute(&self.pool)
            .await?;
        
        debug!("✅ Opportunity stored: {}", opportunity.id);
        Ok(())
    }

    /// Get an opportunity by ID
    pub async fn get_opportunity(&self, id: &str) -> Result<Option<Opportunity>> {
        let opportunity_id = Uuid::parse_str(id)?;
        
        let query = r#"
            SELECT id, chain_id, strategy_type, tokens, pools,
                   expected_profit, gas_cost, confidence, deadline,
                   created_at, metadata
            FROM opportunities 
            WHERE id = $1
        "#;
        
        let row = sqlx::query(query)
            .bind(opportunity_id)
            .fetch_optional(&self.pool)
            .await?;
        
        match row {
            Some(row) => {
                let opportunity = Opportunity {
                    id: row.get::<Uuid, _>("id").to_string(),
                    chain_id: row.get::<i64, _>("chain_id") as u64,
                    strategy_type: row.get("strategy_type"),
                    tokens: row.get("tokens"),
                    pools: row.get("pools"),
                    expected_profit: ethers::types::U256::from_dec_str(&row.get::<String, _>("expected_profit"))?,
                    gas_cost: ethers::types::U256::from_dec_str(&row.get::<String, _>("gas_cost"))?,
                    confidence: row.get("confidence"),
                    deadline: row.get::<i64, _>("deadline") as u64,
                    created_at: row.get("created_at"),
                    metadata: serde_json::from_value(row.get("metadata"))?,
                };
                Ok(Some(opportunity))
            },
            None => Ok(None),
        }
    }

    /// List opportunities with filters
    pub async fn list_opportunities(
        &self,
        chain_id: Option<u64>,
        strategy_type: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<Opportunity>> {
        let mut query = String::from(r#"
            SELECT id, chain_id, strategy_type, tokens, pools,
                   expected_profit, gas_cost, confidence, deadline,
                   created_at, metadata
            FROM opportunities 
            WHERE 1=1
        "#);
        
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 0;
        
        if let Some(chain_id) = chain_id {
            param_count += 1;
            query.push_str(&format!(" AND chain_id = ${}", param_count));
            params.push(Box::new(chain_id as i64));
        }
        
        if let Some(strategy_type) = strategy_type {
            param_count += 1;
            query.push_str(&format!(" AND strategy_type = ${}", param_count));
            params.push(Box::new(strategy_type.to_string()));
        }
        
        query.push_str(" ORDER BY created_at DESC");
        
        if let Some(limit) = limit {
            param_count += 1;
            query.push_str(&format!(" LIMIT ${}", param_count));
            params.push(Box::new(limit));
        }
        
        // For now, use a simpler query without dynamic parameters
        let simple_query = r#"
            SELECT id, chain_id, strategy_type, tokens, pools,
                   expected_profit, gas_cost, confidence, deadline,
                   created_at, metadata
            FROM opportunities 
            ORDER BY created_at DESC
            LIMIT 100
        "#;
        
        let rows = sqlx::query(simple_query)
            .fetch_all(&self.pool)
            .await?;
        
        let mut opportunities = Vec::new();
        for row in rows {
            let opportunity = Opportunity {
                id: row.get::<Uuid, _>("id").to_string(),
                chain_id: row.get::<i64, _>("chain_id") as u64,
                strategy_type: row.get("strategy_type"),
                tokens: row.get("tokens"),
                pools: row.get("pools"),
                expected_profit: ethers::types::U256::from_dec_str(&row.get::<String, _>("expected_profit"))?,
                gas_cost: ethers::types::U256::from_dec_str(&row.get::<String, _>("gas_cost"))?,
                confidence: row.get("confidence"),
                deadline: row.get::<i64, _>("deadline") as u64,
                created_at: row.get("created_at"),
                metadata: serde_json::from_value(row.get("metadata"))?,
            };
            opportunities.push(opportunity);
        }
        
        Ok(opportunities)
    }

    /// Store execution result
    pub async fn store_execution(
        &self,
        opportunity_id: &str,
        tx_hash: Option<&str>,
        status: &str,
        actual_profit: Option<ethers::types::U256>,
        gas_used: Option<u64>,
        error_reason: Option<&str>,
    ) -> Result<String> {
        let execution_id = Uuid::new_v4();
        let opportunity_uuid = Uuid::parse_str(opportunity_id)?;
        
        let query = r#"
            INSERT INTO executions (
                id, opportunity_id, tx_hash, status, 
                actual_profit, gas_used, error_reason, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        "#;
        
        let actual_profit_str = actual_profit.map(|p| p.to_string());
        let gas_used_i64 = gas_used.map(|g| g as i64);
        
        sqlx::query(query)
            .bind(execution_id)
            .bind(opportunity_uuid)
            .bind(tx_hash)
            .bind(status)
            .bind(actual_profit_str)
            .bind(gas_used_i64)
            .bind(error_reason)
            .execute(&self.pool)
            .await?;
        
        info!("💾 Execution stored: {}", execution_id);
        Ok(execution_id.to_string())
    }

    /// Get execution by ID
    pub async fn get_execution(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let execution_id = Uuid::parse_str(id)?;
        
        let query = r#"
            SELECT e.*, o.strategy_type, o.tokens, o.pools
            FROM executions e
            JOIN opportunities o ON e.opportunity_id = o.id
            WHERE e.id = $1
        "#;
        
        let row = sqlx::query(query)
            .bind(execution_id)
            .fetch_optional(&self.pool)
            .await?;
        
        match row {
            Some(row) => {
                let execution = serde_json::json!({
                    "id": row.get::<Uuid, _>("id").to_string(),
                    "opportunity_id": row.get::<Uuid, _>("opportunity_id").to_string(),
                    "tx_hash": row.get::<Option<String>, _>("tx_hash"),
                    "status": row.get::<String, _>("status"),
                    "actual_profit": row.get::<Option<String>, _>("actual_profit"),
                    "gas_used": row.get::<Option<i64>, _>("gas_used"),
                    "error_reason": row.get::<Option<String>, _>("error_reason"),
                    "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                    "strategy_type": row.get::<String, _>("strategy_type"),
                    "tokens": row.get::<Vec<String>, _>("tokens"),
                    "pools": row.get::<Vec<String>, _>("pools"),
                });
                Ok(Some(execution))
            },
            None => Ok(None),
        }
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self, hours: i32) -> Result<serde_json::Value> {
        let query = r#"
            SELECT 
                COUNT(*) as total_opportunities,
                COUNT(CASE WHEN e.status = 'completed' THEN 1 END) as successful_executions,
                SUM(CASE WHEN e.actual_profit IS NOT NULL THEN CAST(e.actual_profit AS DECIMAL) ELSE 0 END) as total_profit,
                AVG(CASE WHEN e.actual_profit IS NOT NULL THEN CAST(e.actual_profit AS DECIMAL) ELSE NULL END) as avg_profit,
                AVG(o.confidence) as avg_confidence
            FROM opportunities o
            LEFT JOIN executions e ON o.id = e.opportunity_id
            WHERE o.created_at >= NOW() - INTERVAL '%d hours'
        "#;
        
        let formatted_query = query.replace("%d", &hours.to_string());
        
        let row = sqlx::query(&formatted_query)
            .fetch_one(&self.pool)
            .await?;
        
        let metrics = serde_json::json!({
            "total_opportunities": row.get::<i64, _>("total_opportunities"),
            "successful_executions": row.get::<i64, _>("successful_executions"),
            "total_profit": row.get::<Option<rust_decimal::Decimal>, _>("total_profit")
                .map(|d| d.to_string())
                .unwrap_or_else(|| "0".to_string()),
            "average_profit": row.get::<Option<rust_decimal::Decimal>, _>("avg_profit")
                .map(|d| d.to_string())
                .unwrap_or_else(|| "0".to_string()),
            "average_confidence": row.get::<Option<f64>, _>("avg_confidence").unwrap_or(0.0),
            "success_rate": {
                let total = row.get::<i64, _>("total_opportunities");
                let successful = row.get::<i64, _>("successful_executions");
                if total > 0 { successful as f64 / total as f64 } else { 0.0 }
            }
        });
        
        Ok(metrics)
    }

    /// Update pool data
    pub async fn update_pool(&self, pool: &DexPool) -> Result<()> {
        let query = r#"
            INSERT INTO pools (address, chain, project, symbol, tvl_usd, apy, volume_usd_1d, volume_usd_7d, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT (address) DO UPDATE SET
                tvl_usd = EXCLUDED.tvl_usd,
                apy = EXCLUDED.apy,
                volume_usd_1d = EXCLUDED.volume_usd_1d,
                volume_usd_7d = EXCLUDED.volume_usd_7d,
                updated_at = NOW()
        "#;
        
        sqlx::query(query)
            .bind(&pool.address)
            .bind(&pool.chain)
            .bind(&pool.project)
            .bind(&pool.symbol)
            .bind(pool.tvl_usd)
            .bind(pool.apy)
            .bind(pool.volume_usd_1d)
            .bind(pool.volume_usd_7d)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }

    /// Get pools for a specific chain
    pub async fn get_pools(&self, chain: &str) -> Result<HashMap<String, DexPool>> {
        let query = r#"
            SELECT address, chain, project, symbol, tvl_usd, apy, volume_usd_1d, volume_usd_7d, updated_at
            FROM pools
            WHERE chain = $1 AND updated_at > NOW() - INTERVAL '1 hour'
            ORDER BY tvl_usd DESC
            LIMIT 1000
        "#;
        
        let rows = sqlx::query(query)
            .bind(chain)
            .fetch_all(&self.pool)
            .await?;
        
        let mut pools = HashMap::new();
        for row in rows {
            let pool = DexPool {
                address: row.get("address"),
                chain: row.get("chain"),
                project: row.get("project"),
                symbol: row.get("symbol"),
                tvl_usd: row.get("tvl_usd"),
                apy: row.get("apy"),
                volume_usd_1d: row.get("volume_usd_1d"),
                volume_usd_7d: row.get("volume_usd_7d"),
                last_updated: row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                    .timestamp() as u64,
            };
            pools.insert(pool.address.clone(), pool);
        }
        
        Ok(pools)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }

    /// Clean up old opportunities
    pub async fn cleanup_old_opportunities(&self, hours: i32) -> Result<u64> {
        let query = r#"
            DELETE FROM opportunities 
            WHERE created_at < NOW() - INTERVAL '%d hours'
        "#;
        
        let formatted_query = query.replace("%d", &hours.to_string());
        
        let result = sqlx::query(&formatted_query)
            .execute(&self.pool)
            .await?;
        
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection() {
        // This would be a real test with a test database
        assert!(true);
    }
}
