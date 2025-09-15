// ArbitrageX Supreme V3.0 - Shared Utilities
// Common utility functions used across all services

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Generate a new UUID v4
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a new request ID for tracing
pub fn generate_request_id() -> String {
    format!("req_{}", Uuid::new_v4().simple())
}

/// Get current timestamp in UTC
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Format timestamp for logging
pub fn format_timestamp(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Calculate time difference in milliseconds
pub fn time_diff_ms(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    (end - start).num_milliseconds()
}

/// Check if timestamp is expired
pub fn is_expired(timestamp: DateTime<Utc>, ttl_seconds: i64) -> bool {
    let now = Utc::now();
    let elapsed = (now - timestamp).num_seconds();
    elapsed > ttl_seconds
}

/// Format decimal as USD currency
pub fn format_usd(amount: Decimal) -> String {
    format!("${:.2}", amount)
}

/// Format decimal as percentage
pub fn format_percentage(value: Decimal) -> String {
    format!("{:.2}%", value * Decimal::from(100))
}

/// Format gas price in Gwei
pub fn format_gas_price(gwei: u64) -> String {
    format!("{} Gwei", gwei)
}

/// Format gas amount
pub fn format_gas(gas: u64) -> String {
    if gas >= 1_000_000 {
        format!("{:.1}M", gas as f64 / 1_000_000.0)
    } else if gas >= 1_000 {
        format!("{:.1}K", gas as f64 / 1_000.0)
    } else {
        gas.to_string()
    }
}

/// Format blockchain address (truncate middle)
pub fn format_address(address: &str) -> String {
    if address.len() > 10 {
        format!("{}...{}", &address[..6], &address[address.len()-4..])
    } else {
        address.to_string()
    }
}

/// Format transaction hash (truncate middle)
pub fn format_tx_hash(hash: &str) -> String {
    if hash.len() > 16 {
        format!("{}...{}", &hash[..8], &hash[hash.len()-8..])
    } else {
        hash.to_string()
    }
}

/// Validate Ethereum address format
pub fn is_valid_eth_address(address: &str) -> bool {
    address.starts_with("0x") && address.len() == 42 && address[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate transaction hash format
pub fn is_valid_tx_hash(hash: &str) -> bool {
    hash.starts_with("0x") && hash.len() == 66 && hash[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate block number
pub fn is_valid_block_number(block: u64) -> bool {
    block > 0 && block < u64::MAX
}

/// Calculate percentage change
pub fn percentage_change(old_value: Decimal, new_value: Decimal) -> Decimal {
    if old_value.is_zero() {
        return Decimal::ZERO;
    }
    ((new_value - old_value) / old_value) * Decimal::from(100)
}

/// Calculate profit margin
pub fn profit_margin(revenue: Decimal, cost: Decimal) -> Decimal {
    if revenue.is_zero() {
        return Decimal::ZERO;
    }
    ((revenue - cost) / revenue) * Decimal::from(100)
}

/// Calculate ROI (Return on Investment)
pub fn calculate_roi(profit: Decimal, investment: Decimal) -> Decimal {
    if investment.is_zero() {
        return Decimal::ZERO;
    }
    (profit / investment) * Decimal::from(100)
}

/// Calculate gas cost in USD
pub fn calculate_gas_cost_usd(gas_used: u64, gas_price_gwei: u64, eth_price_usd: Decimal) -> Decimal {
    let gas_cost_eth = Decimal::from(gas_used) * Decimal::from(gas_price_gwei) / Decimal::from(1_000_000_000);
    gas_cost_eth * eth_price_usd
}

/// Calculate slippage percentage
pub fn calculate_slippage(expected: Decimal, actual: Decimal) -> Decimal {
    if expected.is_zero() {
        return Decimal::ZERO;
    }
    ((expected - actual).abs() / expected) * Decimal::from(100)
}

/// Round to specified decimal places
pub fn round_decimal(value: Decimal, places: u32) -> Decimal {
    value.round_dp(places)
}

/// Truncate string to max length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Sanitize string for logging (remove sensitive data)
pub fn sanitize_for_log(s: &str) -> String {
    // Replace potential private keys, API keys, passwords
    let sensitive_patterns = [
        "private_key", "api_key", "password", "secret", "token",
        "0x[a-fA-F0-9]{64}", // Private keys
        "[A-Za-z0-9]{32,}", // API keys
    ];
    
    let mut sanitized = s.to_string();
    for pattern in &sensitive_patterns {
        if s.to_lowercase().contains(&pattern.to_lowercase()) {
            sanitized = "[REDACTED]".to_string();
            break;
        }
    }
    sanitized
}

/// Convert string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() && !result.is_empty() {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    
    result
}

/// Convert string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    to_snake_case(s).replace('_', "-")
}

/// Parse environment variable with default
pub fn env_var_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Parse environment variable as integer with default
pub fn env_var_as_u64_or_default(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Parse environment variable as boolean with default
pub fn env_var_as_bool_or_default(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(default)
}

/// Create a HashMap from key-value pairs
pub fn create_metadata(pairs: Vec<(&str, Value)>) -> HashMap<String, Value> {
    pairs.into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Merge two HashMaps
pub fn merge_metadata(mut base: HashMap<String, Value>, other: HashMap<String, Value>) -> HashMap<String, Value> {
    for (key, value) in other {
        base.insert(key, value);
    }
    base
}

/// Extract numeric value from JSON Value
pub fn extract_number(value: &Value, key: &str) -> Option<f64> {
    value.get(key)?.as_f64()
}

/// Extract string value from JSON Value
pub fn extract_string(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(|s| s.to_string())
}

/// Extract boolean value from JSON Value
pub fn extract_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key)?.as_bool()
}

/// Retry logic with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    max_retries: u32,
    initial_delay_ms: u64,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut retries = 0;
    let mut delay = initial_delay_ms;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if retries >= max_retries {
                    return Err(error);
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                retries += 1;
                delay *= 2; // Exponential backoff
            }
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    tokens: std::sync::Arc<tokio::sync::Mutex<f64>>,
    capacity: f64,
    refill_rate: f64,
    last_refill: std::sync::Arc<tokio::sync::Mutex<std::time::Instant>>,
}

impl RateLimiter {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: std::sync::Arc::new(tokio::sync::Mutex::new(capacity)),
            capacity,
            refill_rate,
            last_refill: std::sync::Arc::new(tokio::sync::Mutex::new(std::time::Instant::now())),
        }
    }

    pub async fn try_acquire(&self, tokens: f64) -> bool {
        let mut current_tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;
        
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        
        // Refill tokens
        *current_tokens = (*current_tokens + elapsed * self.refill_rate).min(self.capacity);
        *last_refill = now;
        
        if *current_tokens >= tokens {
            *current_tokens -= tokens;
            true
        } else {
            false
        }
    }
}

/// Circuit breaker pattern
pub struct CircuitBreaker {
    failure_count: std::sync::Arc<tokio::sync::Mutex<u32>>,
    last_failure: std::sync::Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    failure_threshold: u32,
    timeout_duration: std::time::Duration,
    state: std::sync::Arc<tokio::sync::Mutex<CircuitBreakerState>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout_duration: std::time::Duration) -> Self {
        Self {
            failure_count: std::sync::Arc::new(tokio::sync::Mutex::new(0)),
            last_failure: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            failure_threshold,
            timeout_duration,
            state: std::sync::Arc::new(tokio::sync::Mutex::new(CircuitBreakerState::Closed)),
        }
    }

    pub async fn call<F, Fut, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let mut state = self.state.lock().await;
        
        match *state {
            CircuitBreakerState::Open => {
                let last_failure = self.last_failure.lock().await;
                if let Some(last_fail_time) = *last_failure {
                    if last_fail_time.elapsed() > self.timeout_duration {
                        *state = CircuitBreakerState::HalfOpen;
                    } else {
                        return operation().await; // Still return error
                    }
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Allow one request to test if service is back
            }
            CircuitBreakerState::Closed => {
                // Normal operation
            }
        }
        
        drop(state);
        
        match operation().await {
            Ok(result) => {
                // Reset on success
                *self.failure_count.lock().await = 0;
                *self.state.lock().await = CircuitBreakerState::Closed;
                Ok(result)
            }
            Err(error) => {
                // Increment failure count
                let mut failure_count = self.failure_count.lock().await;
                *failure_count += 1;
                
                if *failure_count >= self.failure_threshold {
                    *self.state.lock().await = CircuitBreakerState::Open;
                    *self.last_failure.lock().await = Some(std::time::Instant::now());
                }
                
                Err(error)
            }
        }
    }
}

/// Health check utilities
pub fn check_service_health(url: &str) -> impl std::future::Future<Output = bool> + '_ {
    async move {
        match reqwest::get(url).await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

/// Memory usage utilities
pub fn get_memory_usage() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(contents) = fs::read_to_string("/proc/self/status") {
            for line in contents.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return Some(kb * 1024); // Convert to bytes
                        }
                    }
                }
            }
        }
    }
    None
}

/// CPU usage utilities (simplified)
pub fn get_cpu_usage() -> Option<f64> {
    // This is a simplified implementation
    // In production, you'd want to use a proper system monitoring library
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert!(Uuid::parse_str(&id1).is_ok());
    }

    #[test]
    fn test_format_functions() {
        assert_eq!(format_usd(Decimal::from_f64(123.456).unwrap()), "$123.46");
        assert_eq!(format_percentage(Decimal::from_f64(0.1234).unwrap()), "12.34%");
        assert_eq!(format_gas_price(20), "20 Gwei");
        assert_eq!(format_gas(1500000), "1.5M");
        assert_eq!(format_gas(1500), "1.5K");
    }

    #[test]
    fn test_validation_functions() {
        assert!(is_valid_eth_address("0x742d35Cc6634C0532925a3b8D400E4C5C0532925"));
        assert!(!is_valid_eth_address("invalid_address"));
        
        assert!(is_valid_tx_hash("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"));
        assert!(!is_valid_tx_hash("invalid_hash"));
        
        assert!(is_valid_block_number(123456));
        assert!(!is_valid_block_number(0));
    }

    #[test]
    fn test_calculation_functions() {
        let old_val = Decimal::from(100);
        let new_val = Decimal::from(110);
        assert_eq!(percentage_change(old_val, new_val), Decimal::from(10));
        
        let revenue = Decimal::from(1000);
        let cost = Decimal::from(800);
        assert_eq!(profit_margin(revenue, cost), Decimal::from(20));
        
        let profit = Decimal::from(200);
        let investment = Decimal::from(1000);
        assert_eq!(calculate_roi(profit, investment), Decimal::from(20));
    }

    #[test]
    fn test_string_utilities() {
        assert_eq!(to_snake_case("CamelCase"), "camel_case");
        assert_eq!(to_kebab_case("CamelCase"), "camel-case");
        assert_eq!(truncate_string("Hello World", 8), "Hello...");
        assert_eq!(format_address("0x742d35Cc6634C0532925a3b8D400E4C5C0532925"), "0x742d...2925");
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0, 1.0); // 10 tokens, 1 token per second
        
        // Should be able to acquire 10 tokens initially
        for _ in 0..10 {
            assert!(limiter.try_acquire(1.0).await);
        }
        
        // Should fail to acquire more
        assert!(!limiter.try_acquire(1.0).await);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(2, std::time::Duration::from_millis(100));
        
        // Simulate failures
        let result1: Result<(), &str> = breaker.call(|| async { Err("error") }).await;
        assert!(result1.is_err());
        
        let result2: Result<(), &str> = breaker.call(|| async { Err("error") }).await;
        assert!(result2.is_err());
        
        // Circuit should be open now
        let state = breaker.state.lock().await;
        assert_eq!(*state, CircuitBreakerState::Open);
    }
}
