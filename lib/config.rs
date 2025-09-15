// ArbitrageX Supreme V3.0 - Shared Configuration
// Configuration management and environment variable handling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub blockchain: BlockchainConfig,
    pub mev: MevConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub arbitrage: ArbitrageConfig,
    pub services: ServicesConfig,
    pub logging: LoggingConfig,
    pub performance: PerformanceConfig,
    pub features: FeatureFlags,
    pub alerts: AlertConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub max_connections: u32,
    pub timeout_seconds: u64,
    pub cors_origins: Vec<String>,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub migration_path: String,
    pub ssl_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub ethereum_rpc: String,
    pub ethereum_ws: String,
    pub polygon_rpc: String,
    pub arbitrum_rpc: String,
    pub optimism_rpc: String,
    pub base_rpc: String,
    pub bsc_rpc: String,
    pub request_timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevConfig {
    pub flashbots_api_key: String,
    pub flashbots_relay_url: String,
    pub eden_api_key: String,
    pub eden_relay_url: String,
    pub manifold_api_key: String,
    pub manifold_relay_url: String,
    pub bloxroute_api_key: String,
    pub bloxroute_relay_url: String,
    pub bundle_timeout_seconds: u64,
    pub max_bundle_retries: u32,
    pub simulation_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub api_key_length: usize,
    pub password_min_length: usize,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u64,
    pub encryption_key: String,
    pub cors_max_age: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub prometheus_port: u16,
    pub metrics_interval_seconds: u64,
    pub health_check_interval_seconds: u64,
    pub log_level: String,
    pub jaeger_endpoint: Option<String>,
    pub grafana_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    pub min_profit_usd: f64,
    pub max_gas_price_gwei: u64,
    pub max_slippage_percent: f64,
    pub opportunity_timeout_seconds: u64,
    pub execution_timeout_seconds: u64,
    pub max_concurrent_executions: u32,
    pub profit_threshold_multiplier: f64,
    pub gas_estimation_buffer: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub searcher_enabled: bool,
    pub searcher_port: u16,
    pub selector_enabled: bool,
    pub selector_port: u16,
    pub recon_enabled: bool,
    pub recon_port: u16,
    pub relays_client_enabled: bool,
    pub relays_client_port: u16,
    pub api_server_enabled: bool,
    pub api_server_port: u16,
    pub sim_ctl_enabled: bool,
    pub sim_ctl_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
    pub file_path: Option<String>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
    pub compress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub worker_threads: Option<usize>,
    pub blocking_threads: Option<usize>,
    pub max_blocking_threads: Option<usize>,
    pub thread_stack_size: Option<usize>,
    pub enable_io_uring: bool,
    pub tcp_nodelay: bool,
    pub tcp_keepalive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub enable_websocket: bool,
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub enable_rate_limiting: bool,
    pub enable_caching: bool,
    pub enable_compression: bool,
    pub enable_tls: bool,
    pub enable_auth: bool,
    pub enable_simulation: bool,
    pub enable_bundle_merging: bool,
    pub enable_dynamic_gas: bool,
    pub enable_flashloan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub webhook_url: Option<String>,
    pub slack_token: Option<String>,
    pub slack_channel: Option<String>,
    pub discord_webhook: Option<String>,
    pub email_smtp_host: Option<String>,
    pub email_smtp_port: Option<u16>,
    pub email_username: Option<String>,
    pub email_password: Option<String>,
    pub email_recipients: Vec<String>,
    pub alert_cooldown_minutes: u64,
}

impl AppConfig {
    /// Load configuration from environment variables and TOML file
    pub fn load() -> Result<Self, ConfigError> {
        // Try to load from TOML file first
        if let Ok(config) = Self::from_toml_file("config/app.toml") {
            return Ok(config);
        }

        // Fallback to environment variables
        Self::from_env()
    }

    /// Load configuration from TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(e.to_string()))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            server: ServerConfig {
                host: env_var("SERVER_HOST", "0.0.0.0"),
                port: env_var_parse("SERVER_PORT", 8080)?,
                workers: env_var_parse_optional("SERVER_WORKERS")?,
                max_connections: env_var_parse("SERVER_MAX_CONNECTIONS", 1000)?,
                timeout_seconds: env_var_parse("SERVER_TIMEOUT_SECONDS", 30)?,
                cors_origins: env_var_list("CORS_ORIGINS", vec!["*".to_string()]),
                rate_limit_requests: env_var_parse("RATE_LIMIT_REQUESTS", 100)?,
                rate_limit_window_seconds: env_var_parse("RATE_LIMIT_WINDOW_SECONDS", 60)?,
            },
            database: DatabaseConfig {
                url: env_var_required("DATABASE_URL")?,
                max_connections: env_var_parse("DB_MAX_CONNECTIONS", 20)?,
                min_connections: env_var_parse("DB_MIN_CONNECTIONS", 5)?,
                connection_timeout_seconds: env_var_parse("DB_CONNECTION_TIMEOUT", 30)?,
                idle_timeout_seconds: env_var_parse("DB_IDLE_TIMEOUT", 600)?,
                max_lifetime_seconds: env_var_parse("DB_MAX_LIFETIME", 1800)?,
                migration_path: env_var("DB_MIGRATION_PATH", "migrations"),
                ssl_mode: env_var("DB_SSL_MODE", "prefer"),
            },
            redis: RedisConfig {
                url: env_var("REDIS_URL", "redis://localhost:6379"),
                max_connections: env_var_parse("REDIS_MAX_CONNECTIONS", 10)?,
                connection_timeout_seconds: env_var_parse("REDIS_CONNECTION_TIMEOUT", 5)?,
                command_timeout_seconds: env_var_parse("REDIS_COMMAND_TIMEOUT", 5)?,
                retry_attempts: env_var_parse("REDIS_RETRY_ATTEMPTS", 3)?,
                retry_delay_ms: env_var_parse("REDIS_RETRY_DELAY_MS", 100)?,
            },
            blockchain: BlockchainConfig {
                ethereum_rpc: env_var_required("ETHEREUM_RPC_URL")?,
                ethereum_ws: env_var_required("ETHEREUM_WS_URL")?,
                polygon_rpc: env_var("POLYGON_RPC_URL", "https://polygon-rpc.com"),
                arbitrum_rpc: env_var("ARBITRUM_RPC_URL", "https://arb1.arbitrum.io/rpc"),
                optimism_rpc: env_var("OPTIMISM_RPC_URL", "https://mainnet.optimism.io"),
                base_rpc: env_var("BASE_RPC_URL", "https://mainnet.base.org"),
                bsc_rpc: env_var("BSC_RPC_URL", "https://bsc-dataseed.binance.org"),
                request_timeout_seconds: env_var_parse("BLOCKCHAIN_TIMEOUT", 30)?,
                max_retries: env_var_parse("BLOCKCHAIN_MAX_RETRIES", 3)?,
                retry_delay_ms: env_var_parse("BLOCKCHAIN_RETRY_DELAY_MS", 1000)?,
            },
            mev: MevConfig {
                flashbots_api_key: env_var_required("FLASHBOTS_API_KEY")?,
                flashbots_relay_url: env_var("FLASHBOTS_RELAY_URL", "https://relay.flashbots.net"),
                eden_api_key: env_var("EDEN_API_KEY", ""),
                eden_relay_url: env_var("EDEN_RELAY_URL", "https://api.edennetwork.io"),
                manifold_api_key: env_var("MANIFOLD_API_KEY", ""),
                manifold_relay_url: env_var("MANIFOLD_RELAY_URL", "https://api.manifoldfinance.com"),
                bloxroute_api_key: env_var("BLOXROUTE_API_KEY", ""),
                bloxroute_relay_url: env_var("BLOXROUTE_RELAY_URL", "https://mev.api.blxrbdn.com"),
                bundle_timeout_seconds: env_var_parse("BUNDLE_TIMEOUT_SECONDS", 12)?,
                max_bundle_retries: env_var_parse("MAX_BUNDLE_RETRIES", 3)?,
                simulation_enabled: env_var_parse("SIMULATION_ENABLED", true)?,
            },
            security: SecurityConfig {
                jwt_secret: env_var_required("JWT_SECRET")?,
                jwt_expiry_hours: env_var_parse("JWT_EXPIRY_HOURS", 24)?,
                api_key_length: env_var_parse("API_KEY_LENGTH", 32)?,
                password_min_length: env_var_parse("PASSWORD_MIN_LENGTH", 8)?,
                max_login_attempts: env_var_parse("MAX_LOGIN_ATTEMPTS", 5)?,
                lockout_duration_minutes: env_var_parse("LOCKOUT_DURATION_MINUTES", 15)?,
                encryption_key: env_var_required("ENCRYPTION_KEY")?,
                cors_max_age: env_var_parse("CORS_MAX_AGE", 3600)?,
            },
            monitoring: MonitoringConfig {
                prometheus_port: env_var_parse("PROMETHEUS_PORT", 9090)?,
                metrics_interval_seconds: env_var_parse("METRICS_INTERVAL_SECONDS", 10)?,
                health_check_interval_seconds: env_var_parse("HEALTH_CHECK_INTERVAL_SECONDS", 30)?,
                log_level: env_var("LOG_LEVEL", "info"),
                jaeger_endpoint: env_var_optional("JAEGER_ENDPOINT"),
                grafana_url: env_var_optional("GRAFANA_URL"),
            },
            arbitrage: ArbitrageConfig {
                min_profit_usd: env_var_parse("MIN_PROFIT_USD", 1.0)?,
                max_gas_price_gwei: env_var_parse("MAX_GAS_PRICE_GWEI", 100)?,
                max_slippage_percent: env_var_parse("MAX_SLIPPAGE_PERCENT", 5.0)?,
                opportunity_timeout_seconds: env_var_parse("OPPORTUNITY_TIMEOUT_SECONDS", 300)?,
                execution_timeout_seconds: env_var_parse("EXECUTION_TIMEOUT_SECONDS", 60)?,
                max_concurrent_executions: env_var_parse("MAX_CONCURRENT_EXECUTIONS", 10)?,
                profit_threshold_multiplier: env_var_parse("PROFIT_THRESHOLD_MULTIPLIER", 1.2)?,
                gas_estimation_buffer: env_var_parse("GAS_ESTIMATION_BUFFER", 1.1)?,
            },
            services: ServicesConfig {
                searcher_enabled: env_var_parse("SEARCHER_ENABLED", true)?,
                searcher_port: env_var_parse("SEARCHER_PORT", 8001)?,
                selector_enabled: env_var_parse("SELECTOR_ENABLED", true)?,
                selector_port: env_var_parse("SELECTOR_PORT", 8002)?,
                recon_enabled: env_var_parse("RECON_ENABLED", true)?,
                recon_port: env_var_parse("RECON_PORT", 8003)?,
                relays_client_enabled: env_var_parse("RELAYS_CLIENT_ENABLED", true)?,
                relays_client_port: env_var_parse("RELAYS_CLIENT_PORT", 8004)?,
                api_server_enabled: env_var_parse("API_SERVER_ENABLED", true)?,
                api_server_port: env_var_parse("API_SERVER_PORT", 8000)?,
                sim_ctl_enabled: env_var_parse("SIM_CTL_ENABLED", true)?,
                sim_ctl_port: env_var_parse("SIM_CTL_PORT", 8005)?,
            },
            logging: LoggingConfig {
                level: env_var("LOG_LEVEL", "info"),
                format: env_var("LOG_FORMAT", "json"),
                output: env_var("LOG_OUTPUT", "stdout"),
                file_path: env_var_optional("LOG_FILE_PATH"),
                max_file_size_mb: env_var_parse("LOG_MAX_FILE_SIZE_MB", 100)?,
                max_files: env_var_parse("LOG_MAX_FILES", 10)?,
                compress: env_var_parse("LOG_COMPRESS", true)?,
            },
            performance: PerformanceConfig {
                worker_threads: env_var_parse_optional("WORKER_THREADS")?,
                blocking_threads: env_var_parse_optional("BLOCKING_THREADS")?,
                max_blocking_threads: env_var_parse_optional("MAX_BLOCKING_THREADS")?,
                thread_stack_size: env_var_parse_optional("THREAD_STACK_SIZE")?,
                enable_io_uring: env_var_parse("ENABLE_IO_URING", false)?,
                tcp_nodelay: env_var_parse("TCP_NODELAY", true)?,
                tcp_keepalive: env_var_parse("TCP_KEEPALIVE", true)?,
            },
            features: FeatureFlags {
                enable_websocket: env_var_parse("ENABLE_WEBSOCKET", true)?,
                enable_metrics: env_var_parse("ENABLE_METRICS", true)?,
                enable_tracing: env_var_parse("ENABLE_TRACING", true)?,
                enable_rate_limiting: env_var_parse("ENABLE_RATE_LIMITING", true)?,
                enable_caching: env_var_parse("ENABLE_CACHING", true)?,
                enable_compression: env_var_parse("ENABLE_COMPRESSION", true)?,
                enable_tls: env_var_parse("ENABLE_TLS", false)?,
                enable_auth: env_var_parse("ENABLE_AUTH", true)?,
                enable_simulation: env_var_parse("ENABLE_SIMULATION", true)?,
                enable_bundle_merging: env_var_parse("ENABLE_BUNDLE_MERGING", true)?,
                enable_dynamic_gas: env_var_parse("ENABLE_DYNAMIC_GAS", true)?,
                enable_flashloan: env_var_parse("ENABLE_FLASHLOAN", false)?,
            },
            alerts: AlertConfig {
                webhook_url: env_var_optional("ALERT_WEBHOOK_URL"),
                slack_token: env_var_optional("SLACK_TOKEN"),
                slack_channel: env_var_optional("SLACK_CHANNEL"),
                discord_webhook: env_var_optional("DISCORD_WEBHOOK"),
                email_smtp_host: env_var_optional("EMAIL_SMTP_HOST"),
                email_smtp_port: env_var_parse_optional("EMAIL_SMTP_PORT")?,
                email_username: env_var_optional("EMAIL_USERNAME"),
                email_password: env_var_optional("EMAIL_PASSWORD"),
                email_recipients: env_var_list("EMAIL_RECIPIENTS", vec![]),
                alert_cooldown_minutes: env_var_parse("ALERT_COOLDOWN_MINUTES", 5)?,
            },
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate required fields
        if self.database.url.is_empty() {
            return Err(ConfigError::Validation("DATABASE_URL is required".to_string()));
        }

        if self.mev.flashbots_api_key.is_empty() {
            return Err(ConfigError::Validation("FLASHBOTS_API_KEY is required".to_string()));
        }

        if self.security.jwt_secret.len() < 32 {
            return Err(ConfigError::Validation("JWT_SECRET must be at least 32 characters".to_string()));
        }

        // Validate ranges
        if self.server.port == 0 {
            return Err(ConfigError::Validation("SERVER_PORT must be greater than 0".to_string()));
        }

        if self.arbitrage.min_profit_usd < 0.0 {
            return Err(ConfigError::Validation("MIN_PROFIT_USD must be positive".to_string()));
        }

        if self.arbitrage.max_slippage_percent < 0.0 || self.arbitrage.max_slippage_percent > 100.0 {
            return Err(ConfigError::Validation("MAX_SLIPPAGE_PERCENT must be between 0 and 100".to_string()));
        }

        Ok(())
    }
}

/// Configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileRead(String),
    
    #[error("Failed to parse config: {0}")]
    Parse(String),
    
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    
    #[error("Invalid environment variable value: {0}")]
    InvalidEnvVar(String),
    
    #[error("Configuration validation error: {0}")]
    Validation(String),
}

/// Helper functions for environment variable parsing
fn env_var(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_var_required(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingEnvVar(key.to_string()))
}

fn env_var_optional(key: &str) -> Option<String> {
    env::var(key).ok()
}

fn env_var_parse<T: std::str::FromStr>(key: &str, default: T) -> Result<T, ConfigError>
where
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(val) => val.parse().map_err(|e| ConfigError::InvalidEnvVar(format!("{}: {}", key, e))),
        Err(_) => Ok(default),
    }
}

fn env_var_parse_optional<T: std::str::FromStr>(key: &str) -> Result<Option<T>, ConfigError>
where
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(val) => val.parse().map(Some).map_err(|e| ConfigError::InvalidEnvVar(format!("{}: {}", key, e))),
        Err(_) => Ok(None),
    }
}

fn env_var_list(key: &str, default: Vec<String>) -> Vec<String> {
    match env::var(key) {
        Ok(val) => val.split(',').map(|s| s.trim().to_string()).collect(),
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_var_helpers() {
        env::set_var("TEST_VAR", "test_value");
        assert_eq!(env_var("TEST_VAR", "default"), "test_value");
        assert_eq!(env_var("NON_EXISTENT", "default"), "default");

        env::set_var("TEST_NUM", "42");
        assert_eq!(env_var_parse("TEST_NUM", 0).unwrap(), 42);
        assert_eq!(env_var_parse("NON_EXISTENT_NUM", 99).unwrap(), 99);

        env::set_var("TEST_LIST", "a,b,c");
        assert_eq!(env_var_list("TEST_LIST", vec![]), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: 8080,
                workers: None,
                max_connections: 1000,
                timeout_seconds: 30,
                cors_origins: vec!["*".to_string()],
                rate_limit_requests: 100,
                rate_limit_window_seconds: 60,
            },
            database: DatabaseConfig {
                url: "postgresql://localhost/test".to_string(),
                max_connections: 20,
                min_connections: 5,
                connection_timeout_seconds: 30,
                idle_timeout_seconds: 600,
                max_lifetime_seconds: 1800,
                migration_path: "migrations".to_string(),
                ssl_mode: "prefer".to_string(),
            },
            security: SecurityConfig {
                jwt_secret: "a_very_long_secret_key_that_is_secure".to_string(),
                jwt_expiry_hours: 24,
                api_key_length: 32,
                password_min_length: 8,
                max_login_attempts: 5,
                lockout_duration_minutes: 15,
                encryption_key: "encryption_key".to_string(),
                cors_max_age: 3600,
            },
            arbitrage: ArbitrageConfig {
                min_profit_usd: 1.0,
                max_gas_price_gwei: 100,
                max_slippage_percent: 5.0,
                opportunity_timeout_seconds: 300,
                execution_timeout_seconds: 60,
                max_concurrent_executions: 10,
                profit_threshold_multiplier: 1.2,
                gas_estimation_buffer: 1.1,
            },
            // ... other fields with default values
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
                connection_timeout_seconds: 5,
                command_timeout_seconds: 5,
                retry_attempts: 3,
                retry_delay_ms: 100,
            },
            blockchain: BlockchainConfig {
                ethereum_rpc: "https://mainnet.infura.io/v3/key".to_string(),
                ethereum_ws: "wss://mainnet.infura.io/ws/v3/key".to_string(),
                polygon_rpc: "https://polygon-rpc.com".to_string(),
                arbitrum_rpc: "https://arb1.arbitrum.io/rpc".to_string(),
                optimism_rpc: "https://mainnet.optimism.io".to_string(),
                base_rpc: "https://mainnet.base.org".to_string(),
                bsc_rpc: "https://bsc-dataseed.binance.org".to_string(),
                request_timeout_seconds: 30,
                max_retries: 3,
                retry_delay_ms: 1000,
            },
            mev: MevConfig {
                flashbots_api_key: "test_key".to_string(),
                flashbots_relay_url: "https://relay.flashbots.net".to_string(),
                eden_api_key: "".to_string(),
                eden_relay_url: "https://api.edennetwork.io".to_string(),
                manifold_api_key: "".to_string(),
                manifold_relay_url: "https://api.manifoldfinance.com".to_string(),
                bloxroute_api_key: "".to_string(),
                bloxroute_relay_url: "https://mev.api.blxrbdn.com".to_string(),
                bundle_timeout_seconds: 12,
                max_bundle_retries: 3,
                simulation_enabled: true,
            },
            monitoring: MonitoringConfig {
                prometheus_port: 9090,
                metrics_interval_seconds: 10,
                health_check_interval_seconds: 30,
                log_level: "info".to_string(),
                jaeger_endpoint: None,
                grafana_url: None,
            },
            services: ServicesConfig {
                searcher_enabled: true,
                searcher_port: 8001,
                selector_enabled: true,
                selector_port: 8002,
                recon_enabled: true,
                recon_port: 8003,
                relays_client_enabled: true,
                relays_client_port: 8004,
                api_server_enabled: true,
                api_server_port: 8000,
                sim_ctl_enabled: true,
                sim_ctl_port: 8005,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                output: "stdout".to_string(),
                file_path: None,
                max_file_size_mb: 100,
                max_files: 10,
                compress: true,
            },
            performance: PerformanceConfig {
                worker_threads: None,
                blocking_threads: None,
                max_blocking_threads: None,
                thread_stack_size: None,
                enable_io_uring: false,
                tcp_nodelay: true,
                tcp_keepalive: true,
            },
            features: FeatureFlags {
                enable_websocket: true,
                enable_metrics: true,
                enable_tracing: true,
                enable_rate_limiting: true,
                enable_caching: true,
                enable_compression: true,
                enable_tls: false,
                enable_auth: true,
                enable_simulation: true,
                enable_bundle_merging: true,
                enable_dynamic_gas: true,
                enable_flashloan: false,
            },
            alerts: AlertConfig {
                webhook_url: None,
                slack_token: None,
                slack_channel: None,
                discord_webhook: None,
                email_smtp_host: None,
                email_smtp_port: None,
                email_username: None,
                email_password: None,
                email_recipients: vec![],
                alert_cooldown_minutes: 5,
            },
        };

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid port should fail
        config.server.port = 0;
        assert!(config.validate().is_err());

        // Reset port and test invalid slippage
        config.server.port = 8080;
        config.arbitrage.max_slippage_percent = 150.0;
        assert!(config.validate().is_err());
    }
}
