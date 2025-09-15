// ArbitrageX Supreme V3.0 - Error Handling
// Centralized error types and handling for all services

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Main error type for ArbitrageX operations
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ArbitrageError {
    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Redis error: {message}")]
    Redis { message: String },

    #[error("Blockchain RPC error: {message}")]
    BlockchainRpc { message: String },

    #[error("MEV relay error: {relay} - {message}")]
    MevRelay { relay: String, message: String },

    #[error("Opportunity error: {message}")]
    Opportunity { message: String },

    #[error("Execution error: {message}")]
    Execution { message: String },

    #[error("Authentication error: {message}")]
    Authentication { message: String },

    #[error("Authorization error: {message}")]
    Authorization { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Timeout error: {operation} timed out after {seconds}s")]
    Timeout { operation: String, seconds: u64 },

    #[error("Rate limit exceeded: {limit} requests per {window}")]
    RateLimit { limit: u32, window: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Resource already exists: {resource}")]
    AlreadyExists { resource: String },

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },

    #[error("Gas price too high: {actual_gwei} > {max_gwei} gwei")]
    GasPriceTooHigh { actual_gwei: u64, max_gwei: u64 },

    #[error("Slippage too high: {actual_percent}% > {max_percent}%")]
    SlippageTooHigh { actual_percent: f64, max_percent: f64 },

    #[error("Profit too low: ${actual} < ${minimum} minimum")]
    ProfitTooLow { actual: f64, minimum: f64 },

    #[error("Simulation failed: {message}")]
    SimulationFailed { message: String },

    #[error("Bundle submission failed: {message}")]
    BundleSubmissionFailed { message: String },

    #[error("Reconciliation failed: {message}")]
    ReconciliationFailed { message: String },

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Deserialization error: {message}")]
    Deserialization { message: String },

    #[error("Parsing error: {message}")]
    Parsing { message: String },

    #[error("IO error: {message}")]
    Io { message: String },

    #[error("Crypto error: {message}")]
    Crypto { message: String },

    #[error("Contract error: {contract} - {message}")]
    Contract { contract: String, message: String },

    #[error("Transaction failed: {tx_hash} - {message}")]
    TransactionFailed { tx_hash: String, message: String },

    #[error("Block not found: {block_number}")]
    BlockNotFound { block_number: u64 },

    #[error("Chain not supported: {chain}")]
    ChainNotSupported { chain: String },

    #[error("Token not supported: {token}")]
    TokenNotSupported { token: String },

    #[error("DEX not supported: {dex}")]
    DexNotSupported { dex: String },

    #[error("Feature not enabled: {feature}")]
    FeatureNotEnabled { feature: String },

    #[error("Maintenance mode: {message}")]
    MaintenanceMode { message: String },

    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("Quota exceeded: {quota_type}")]
    QuotaExceeded { quota_type: String },

    #[error("Deprecated: {message}")]
    Deprecated { message: String },

    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

/// Error severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Error context for better debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub service: String,
    pub operation: String,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub chain: Option<String>,
    pub opportunity_id: Option<String>,
    pub execution_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub severity: ErrorSeverity,
    pub retryable: bool,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl ErrorContext {
    pub fn new(service: &str, operation: &str) -> Self {
        Self {
            service: service.to_string(),
            operation: operation.to_string(),
            request_id: None,
            user_id: None,
            chain: None,
            opportunity_id: None,
            execution_id: None,
            timestamp: chrono::Utc::now(),
            severity: ErrorSeverity::Medium,
            retryable: false,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_chain(mut self, chain: String) -> Self {
        self.chain = Some(chain);
        self
    }

    pub fn with_opportunity_id(mut self, opportunity_id: String) -> Self {
        self.opportunity_id = Some(opportunity_id);
        self
    }

    pub fn with_execution_id(mut self, execution_id: String) -> Self {
        self.execution_id = Some(execution_id);
        self
    }

    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Enhanced error with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualError {
    pub error: ArbitrageError,
    pub context: ErrorContext,
    pub source: Option<String>,
    pub stack_trace: Option<String>,
}

impl ContextualError {
    pub fn new(error: ArbitrageError, context: ErrorContext) -> Self {
        Self {
            error,
            context,
            source: None,
            stack_trace: None,
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_stack_trace(mut self, stack_trace: String) -> Self {
        self.stack_trace = Some(stack_trace);
        self
    }

    pub fn is_retryable(&self) -> bool {
        self.context.retryable || matches!(
            self.error,
            ArbitrageError::Network { .. }
                | ArbitrageError::Timeout { .. }
                | ArbitrageError::ServiceUnavailable { .. }
                | ArbitrageError::ExternalService { .. }
        )
    }

    pub fn should_alert(&self) -> bool {
        matches!(
            self.context.severity,
            ErrorSeverity::High | ErrorSeverity::Critical
        )
    }
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}:{}] {} (severity: {:?}, retryable: {})",
            self.context.service,
            self.context.operation,
            self.error,
            self.context.severity,
            self.is_retryable()
        )
    }
}

impl std::error::Error for ContextualError {}

/// Result type alias for ArbitrageX operations
pub type ArbitrageResult<T> = Result<T, ArbitrageError>;

/// Result type alias with context
pub type ContextualResult<T> = Result<T, ContextualError>;

/// Helper macros for creating errors
#[macro_export]
macro_rules! database_error {
    ($msg:expr) => {
        ArbitrageError::Database {
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! validation_error {
    ($field:expr, $msg:expr) => {
        ArbitrageError::Validation {
            field: $field.to_string(),
            message: $msg.to_string(),
        }
    };
}

#[macro_export]
macro_rules! not_found_error {
    ($resource:expr) => {
        ArbitrageError::NotFound {
            resource: $resource.to_string(),
        }
    };
}

#[macro_export]
macro_rules! internal_error {
    ($msg:expr) => {
        ArbitrageError::Internal {
            message: $msg.to_string(),
        }
    };
}

/// Error conversion implementations
impl From<sqlx::Error> for ArbitrageError {
    fn from(err: sqlx::Error) -> Self {
        ArbitrageError::Database {
            message: err.to_string(),
        }
    }
}

impl From<redis::RedisError> for ArbitrageError {
    fn from(err: redis::RedisError) -> Self {
        ArbitrageError::Redis {
            message: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for ArbitrageError {
    fn from(err: reqwest::Error) -> Self {
        ArbitrageError::Network {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ArbitrageError {
    fn from(err: serde_json::Error) -> Self {
        ArbitrageError::Serialization {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ArbitrageError {
    fn from(err: std::io::Error) -> Self {
        ArbitrageError::Io {
            message: err.to_string(),
        }
    }
}

impl From<tokio::time::error::Elapsed> for ArbitrageError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        ArbitrageError::Timeout {
            operation: "unknown".to_string(),
            seconds: 0,
        }
    }
}

impl From<anyhow::Error> for ArbitrageError {
    fn from(err: anyhow::Error) -> Self {
        ArbitrageError::Internal {
            message: err.to_string(),
        }
    }
}

/// HTTP status code mapping
impl ArbitrageError {
    pub fn status_code(&self) -> u16 {
        match self {
            ArbitrageError::Authentication { .. } => 401,
            ArbitrageError::Authorization { .. } => 403,
            ArbitrageError::NotFound { .. } => 404,
            ArbitrageError::AlreadyExists { .. } => 409,
            ArbitrageError::Validation { .. } => 400,
            ArbitrageError::RateLimit { .. } => 429,
            ArbitrageError::Timeout { .. } => 408,
            ArbitrageError::ServiceUnavailable { .. } => 503,
            ArbitrageError::MaintenanceMode { .. } => 503,
            ArbitrageError::Internal { .. } => 500,
            _ => 500,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            ArbitrageError::Database { .. } => "DATABASE_ERROR",
            ArbitrageError::Redis { .. } => "REDIS_ERROR",
            ArbitrageError::BlockchainRpc { .. } => "BLOCKCHAIN_RPC_ERROR",
            ArbitrageError::MevRelay { .. } => "MEV_RELAY_ERROR",
            ArbitrageError::Opportunity { .. } => "OPPORTUNITY_ERROR",
            ArbitrageError::Execution { .. } => "EXECUTION_ERROR",
            ArbitrageError::Authentication { .. } => "AUTHENTICATION_ERROR",
            ArbitrageError::Authorization { .. } => "AUTHORIZATION_ERROR",
            ArbitrageError::Configuration { .. } => "CONFIGURATION_ERROR",
            ArbitrageError::Validation { .. } => "VALIDATION_ERROR",
            ArbitrageError::Network { .. } => "NETWORK_ERROR",
            ArbitrageError::Timeout { .. } => "TIMEOUT_ERROR",
            ArbitrageError::RateLimit { .. } => "RATE_LIMIT_ERROR",
            ArbitrageError::NotFound { .. } => "NOT_FOUND_ERROR",
            ArbitrageError::AlreadyExists { .. } => "ALREADY_EXISTS_ERROR",
            ArbitrageError::InsufficientBalance { .. } => "INSUFFICIENT_BALANCE_ERROR",
            ArbitrageError::GasPriceTooHigh { .. } => "GAS_PRICE_TOO_HIGH_ERROR",
            ArbitrageError::SlippageTooHigh { .. } => "SLIPPAGE_TOO_HIGH_ERROR",
            ArbitrageError::ProfitTooLow { .. } => "PROFIT_TOO_LOW_ERROR",
            ArbitrageError::SimulationFailed { .. } => "SIMULATION_FAILED_ERROR",
            ArbitrageError::BundleSubmissionFailed { .. } => "BUNDLE_SUBMISSION_FAILED_ERROR",
            ArbitrageError::ReconciliationFailed { .. } => "RECONCILIATION_FAILED_ERROR",
            ArbitrageError::Internal { .. } => "INTERNAL_ERROR",
            ArbitrageError::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            ArbitrageError::Serialization { .. } => "SERIALIZATION_ERROR",
            ArbitrageError::Deserialization { .. } => "DESERIALIZATION_ERROR",
            ArbitrageError::Parsing { .. } => "PARSING_ERROR",
            ArbitrageError::Io { .. } => "IO_ERROR",
            ArbitrageError::Crypto { .. } => "CRYPTO_ERROR",
            ArbitrageError::Contract { .. } => "CONTRACT_ERROR",
            ArbitrageError::TransactionFailed { .. } => "TRANSACTION_FAILED_ERROR",
            ArbitrageError::BlockNotFound { .. } => "BLOCK_NOT_FOUND_ERROR",
            ArbitrageError::ChainNotSupported { .. } => "CHAIN_NOT_SUPPORTED_ERROR",
            ArbitrageError::TokenNotSupported { .. } => "TOKEN_NOT_SUPPORTED_ERROR",
            ArbitrageError::DexNotSupported { .. } => "DEX_NOT_SUPPORTED_ERROR",
            ArbitrageError::FeatureNotEnabled { .. } => "FEATURE_NOT_ENABLED_ERROR",
            ArbitrageError::MaintenanceMode { .. } => "MAINTENANCE_MODE_ERROR",
            ArbitrageError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE_ERROR",
            ArbitrageError::VersionMismatch { .. } => "VERSION_MISMATCH_ERROR",
            ArbitrageError::QuotaExceeded { .. } => "QUOTA_EXCEEDED_ERROR",
            ArbitrageError::Deprecated { .. } => "DEPRECATED_ERROR",
            ArbitrageError::Unknown { .. } => "UNKNOWN_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ArbitrageError::Database {
            message: "Connection failed".to_string(),
        };
        assert_eq!(error.status_code(), 500);
        assert_eq!(error.error_code(), "DATABASE_ERROR");
    }

    #[test]
    fn test_contextual_error() {
        let context = ErrorContext::new("api-server", "get_opportunities")
            .with_severity(ErrorSeverity::High)
            .with_retryable(true);

        let error = ArbitrageError::Database {
            message: "Connection failed".to_string(),
        };

        let contextual_error = ContextualError::new(error, context);
        assert!(contextual_error.is_retryable());
        assert!(contextual_error.should_alert());
    }

    #[test]
    fn test_error_macros() {
        let db_error = database_error!("Test error");
        assert!(matches!(db_error, ArbitrageError::Database { .. }));

        let validation_error = validation_error!("email", "Invalid format");
        assert!(matches!(validation_error, ArbitrageError::Validation { .. }));

        let not_found_error = not_found_error!("user");
        assert!(matches!(not_found_error, ArbitrageError::NotFound { .. }));
    }
}
