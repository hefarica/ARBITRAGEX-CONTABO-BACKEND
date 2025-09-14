use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: i64,
    pub max_opportunities_per_page: i64,
    pub ws_heartbeat_interval: u64,
    pub enable_metrics: bool,
}

impl Config {
    pub async fn from_env() -> Result<Self> {
        Ok(Config {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://arbitragex:password@postgres-db:5432/arbitragex".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://redis-cache:6379".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-jwt-key-change-in-production".to_string()),
            jwt_expiration: std::env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string()) // 24 hours
                .parse()?,
            max_opportunities_per_page: 50,
            ws_heartbeat_interval: 30, // seconds
            enable_metrics: std::env::var("ENABLE_METRICS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
        })
    }
}



