// ArbitrageX Supreme V3.0 - Shared Metrics Module
// Prometheus metrics collection and reporting

use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metrics collector for ArbitrageX system
#[derive(Clone)]
pub struct MetricsCollector {
    registry: Arc<Registry>,
    
    // Opportunity metrics
    pub opportunities_total: IntCounter,
    pub opportunities_by_chain: IntCounterVec,
    pub opportunities_by_status: IntCounterVec,
    pub opportunity_profit_usd: Histogram,
    pub opportunity_gas_estimate: Histogram,
    pub opportunity_processing_duration: HistogramVec,
    
    // Execution metrics
    pub executions_total: IntCounter,
    pub executions_by_status: IntCounterVec,
    pub execution_profit_usd: Histogram,
    pub execution_gas_used: Histogram,
    pub execution_duration: HistogramVec,
    pub execution_success_rate: Gauge,
    
    // MEV relay metrics
    pub bundle_submissions_total: IntCounterVec,
    pub bundle_success_rate: GaugeVec,
    pub bundle_response_time: HistogramVec,
    pub relay_errors: IntCounterVec,
    
    // System metrics
    pub active_connections: IntGauge,
    pub memory_usage_bytes: IntGauge,
    pub cpu_usage_percent: Gauge,
    pub database_connections: IntGauge,
    pub redis_connections: IntGauge,
    
    // Business metrics
    pub total_profit_usd: Counter,
    pub total_gas_cost_usd: Counter,
    pub net_profit_usd: Gauge,
    pub arbitrage_volume_usd: Counter,
    pub unique_tokens_traded: IntGauge,
    
    // Performance metrics
    pub http_requests_total: IntCounterVec,
    pub http_request_duration: HistogramVec,
    pub websocket_connections: IntGauge,
    pub websocket_messages: IntCounterVec,
    
    // Error metrics
    pub errors_total: IntCounterVec,
    pub error_rate: GaugeVec,
    
    // Custom metrics storage
    custom_metrics: Arc<RwLock<HashMap<String, Box<dyn prometheus::core::Metric + Send + Sync>>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let registry = Arc::new(Registry::new());
        
        // Opportunity metrics
        let opportunities_total = IntCounter::new(
            "arbitragex_opportunities_total",
            "Total number of arbitrage opportunities detected"
        ).expect("Failed to create opportunities_total metric");
        
        let opportunities_by_chain = IntCounterVec::new(
            Opts::new("arbitragex_opportunities_by_chain", "Opportunities by blockchain"),
            &["chain"]
        ).expect("Failed to create opportunities_by_chain metric");
        
        let opportunities_by_status = IntCounterVec::new(
            Opts::new("arbitragex_opportunities_by_status", "Opportunities by status"),
            &["status"]
        ).expect("Failed to create opportunities_by_status metric");
        
        let opportunity_profit_usd = Histogram::with_opts(
            HistogramOpts::new("arbitragex_opportunity_profit_usd", "Opportunity profit in USD")
                .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
        ).expect("Failed to create opportunity_profit_usd metric");
        
        let opportunity_gas_estimate = Histogram::with_opts(
            HistogramOpts::new("arbitragex_opportunity_gas_estimate", "Opportunity gas estimate")
                .buckets(vec![50000.0, 100000.0, 150000.0, 200000.0, 300000.0, 500000.0])
        ).expect("Failed to create opportunity_gas_estimate metric");
        
        let opportunity_processing_duration = HistogramVec::new(
            HistogramOpts::new("arbitragex_opportunity_processing_duration_seconds", "Opportunity processing duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["stage"]
        ).expect("Failed to create opportunity_processing_duration metric");
        
        // Execution metrics
        let executions_total = IntCounter::new(
            "arbitragex_executions_total",
            "Total number of arbitrage executions"
        ).expect("Failed to create executions_total metric");
        
        let executions_by_status = IntCounterVec::new(
            Opts::new("arbitragex_executions_by_status", "Executions by status"),
            &["status"]
        ).expect("Failed to create executions_by_status metric");
        
        let execution_profit_usd = Histogram::with_opts(
            HistogramOpts::new("arbitragex_execution_profit_usd", "Execution profit in USD")
                .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0])
        ).expect("Failed to create execution_profit_usd metric");
        
        let execution_gas_used = Histogram::with_opts(
            HistogramOpts::new("arbitragex_execution_gas_used", "Execution gas used")
                .buckets(vec![50000.0, 100000.0, 150000.0, 200000.0, 300000.0, 500000.0])
        ).expect("Failed to create execution_gas_used metric");
        
        let execution_duration = HistogramVec::new(
            HistogramOpts::new("arbitragex_execution_duration_seconds", "Execution duration")
                .buckets(vec![1.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0]),
            &["chain"]
        ).expect("Failed to create execution_duration metric");
        
        let execution_success_rate = Gauge::new(
            "arbitragex_execution_success_rate",
            "Execution success rate (0-1)"
        ).expect("Failed to create execution_success_rate metric");
        
        // MEV relay metrics
        let bundle_submissions_total = IntCounterVec::new(
            Opts::new("arbitragex_bundle_submissions_total", "Total bundle submissions"),
            &["relay", "status"]
        ).expect("Failed to create bundle_submissions_total metric");
        
        let bundle_success_rate = GaugeVec::new(
            Opts::new("arbitragex_bundle_success_rate", "Bundle success rate by relay"),
            &["relay"]
        ).expect("Failed to create bundle_success_rate metric");
        
        let bundle_response_time = HistogramVec::new(
            HistogramOpts::new("arbitragex_bundle_response_time_seconds", "Bundle response time")
                .buckets(vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            &["relay"]
        ).expect("Failed to create bundle_response_time metric");
        
        let relay_errors = IntCounterVec::new(
            Opts::new("arbitragex_relay_errors_total", "Relay errors"),
            &["relay", "error_type"]
        ).expect("Failed to create relay_errors metric");
        
        // System metrics
        let active_connections = IntGauge::new(
            "arbitragex_active_connections",
            "Number of active connections"
        ).expect("Failed to create active_connections metric");
        
        let memory_usage_bytes = IntGauge::new(
            "arbitragex_memory_usage_bytes",
            "Memory usage in bytes"
        ).expect("Failed to create memory_usage_bytes metric");
        
        let cpu_usage_percent = Gauge::new(
            "arbitragex_cpu_usage_percent",
            "CPU usage percentage"
        ).expect("Failed to create cpu_usage_percent metric");
        
        let database_connections = IntGauge::new(
            "arbitragex_database_connections",
            "Number of database connections"
        ).expect("Failed to create database_connections metric");
        
        let redis_connections = IntGauge::new(
            "arbitragex_redis_connections",
            "Number of Redis connections"
        ).expect("Failed to create redis_connections metric");
        
        // Business metrics
        let total_profit_usd = Counter::new(
            "arbitragex_total_profit_usd",
            "Total profit in USD"
        ).expect("Failed to create total_profit_usd metric");
        
        let total_gas_cost_usd = Counter::new(
            "arbitragex_total_gas_cost_usd",
            "Total gas cost in USD"
        ).expect("Failed to create total_gas_cost_usd metric");
        
        let net_profit_usd = Gauge::new(
            "arbitragex_net_profit_usd",
            "Net profit in USD"
        ).expect("Failed to create net_profit_usd metric");
        
        let arbitrage_volume_usd = Counter::new(
            "arbitragex_arbitrage_volume_usd",
            "Total arbitrage volume in USD"
        ).expect("Failed to create arbitrage_volume_usd metric");
        
        let unique_tokens_traded = IntGauge::new(
            "arbitragex_unique_tokens_traded",
            "Number of unique tokens traded"
        ).expect("Failed to create unique_tokens_traded metric");
        
        // Performance metrics
        let http_requests_total = IntCounterVec::new(
            Opts::new("arbitragex_http_requests_total", "Total HTTP requests"),
            &["method", "endpoint", "status"]
        ).expect("Failed to create http_requests_total metric");
        
        let http_request_duration = HistogramVec::new(
            HistogramOpts::new("arbitragex_http_request_duration_seconds", "HTTP request duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["method", "endpoint"]
        ).expect("Failed to create http_request_duration metric");
        
        let websocket_connections = IntGauge::new(
            "arbitragex_websocket_connections",
            "Number of WebSocket connections"
        ).expect("Failed to create websocket_connections metric");
        
        let websocket_messages = IntCounterVec::new(
            Opts::new("arbitragex_websocket_messages_total", "WebSocket messages"),
            &["type", "direction"]
        ).expect("Failed to create websocket_messages metric");
        
        // Error metrics
        let errors_total = IntCounterVec::new(
            Opts::new("arbitragex_errors_total", "Total errors"),
            &["service", "error_type", "severity"]
        ).expect("Failed to create errors_total metric");
        
        let error_rate = GaugeVec::new(
            Opts::new("arbitragex_error_rate", "Error rate by service"),
            &["service"]
        ).expect("Failed to create error_rate metric");
        
        // Register all metrics
        registry.register(Box::new(opportunities_total.clone())).expect("Failed to register opportunities_total");
        registry.register(Box::new(opportunities_by_chain.clone())).expect("Failed to register opportunities_by_chain");
        registry.register(Box::new(opportunities_by_status.clone())).expect("Failed to register opportunities_by_status");
        registry.register(Box::new(opportunity_profit_usd.clone())).expect("Failed to register opportunity_profit_usd");
        registry.register(Box::new(opportunity_gas_estimate.clone())).expect("Failed to register opportunity_gas_estimate");
        registry.register(Box::new(opportunity_processing_duration.clone())).expect("Failed to register opportunity_processing_duration");
        
        registry.register(Box::new(executions_total.clone())).expect("Failed to register executions_total");
        registry.register(Box::new(executions_by_status.clone())).expect("Failed to register executions_by_status");
        registry.register(Box::new(execution_profit_usd.clone())).expect("Failed to register execution_profit_usd");
        registry.register(Box::new(execution_gas_used.clone())).expect("Failed to register execution_gas_used");
        registry.register(Box::new(execution_duration.clone())).expect("Failed to register execution_duration");
        registry.register(Box::new(execution_success_rate.clone())).expect("Failed to register execution_success_rate");
        
        registry.register(Box::new(bundle_submissions_total.clone())).expect("Failed to register bundle_submissions_total");
        registry.register(Box::new(bundle_success_rate.clone())).expect("Failed to register bundle_success_rate");
        registry.register(Box::new(bundle_response_time.clone())).expect("Failed to register bundle_response_time");
        registry.register(Box::new(relay_errors.clone())).expect("Failed to register relay_errors");
        
        registry.register(Box::new(active_connections.clone())).expect("Failed to register active_connections");
        registry.register(Box::new(memory_usage_bytes.clone())).expect("Failed to register memory_usage_bytes");
        registry.register(Box::new(cpu_usage_percent.clone())).expect("Failed to register cpu_usage_percent");
        registry.register(Box::new(database_connections.clone())).expect("Failed to register database_connections");
        registry.register(Box::new(redis_connections.clone())).expect("Failed to register redis_connections");
        
        registry.register(Box::new(total_profit_usd.clone())).expect("Failed to register total_profit_usd");
        registry.register(Box::new(total_gas_cost_usd.clone())).expect("Failed to register total_gas_cost_usd");
        registry.register(Box::new(net_profit_usd.clone())).expect("Failed to register net_profit_usd");
        registry.register(Box::new(arbitrage_volume_usd.clone())).expect("Failed to register arbitrage_volume_usd");
        registry.register(Box::new(unique_tokens_traded.clone())).expect("Failed to register unique_tokens_traded");
        
        registry.register(Box::new(http_requests_total.clone())).expect("Failed to register http_requests_total");
        registry.register(Box::new(http_request_duration.clone())).expect("Failed to register http_request_duration");
        registry.register(Box::new(websocket_connections.clone())).expect("Failed to register websocket_connections");
        registry.register(Box::new(websocket_messages.clone())).expect("Failed to register websocket_messages");
        
        registry.register(Box::new(errors_total.clone())).expect("Failed to register errors_total");
        registry.register(Box::new(error_rate.clone())).expect("Failed to register error_rate");
        
        Self {
            registry,
            opportunities_total,
            opportunities_by_chain,
            opportunities_by_status,
            opportunity_profit_usd,
            opportunity_gas_estimate,
            opportunity_processing_duration,
            executions_total,
            executions_by_status,
            execution_profit_usd,
            execution_gas_used,
            execution_duration,
            execution_success_rate,
            bundle_submissions_total,
            bundle_success_rate,
            bundle_response_time,
            relay_errors,
            active_connections,
            memory_usage_bytes,
            cpu_usage_percent,
            database_connections,
            redis_connections,
            total_profit_usd,
            total_gas_cost_usd,
            net_profit_usd,
            arbitrage_volume_usd,
            unique_tokens_traded,
            http_requests_total,
            http_request_duration,
            websocket_connections,
            websocket_messages,
            errors_total,
            error_rate,
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get the Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
    
    /// Record an opportunity detection
    pub fn record_opportunity(&self, chain: &str, status: &str, profit_usd: f64, gas_estimate: u64) {
        self.opportunities_total.inc();
        self.opportunities_by_chain.with_label_values(&[chain]).inc();
        self.opportunities_by_status.with_label_values(&[status]).inc();
        self.opportunity_profit_usd.observe(profit_usd);
        self.opportunity_gas_estimate.observe(gas_estimate as f64);
    }
    
    /// Record opportunity processing time
    pub fn record_opportunity_processing(&self, stage: &str, duration_seconds: f64) {
        self.opportunity_processing_duration
            .with_label_values(&[stage])
            .observe(duration_seconds);
    }
    
    /// Record an execution
    pub fn record_execution(&self, chain: &str, status: &str, profit_usd: Option<f64>, gas_used: Option<u64>, duration_seconds: f64) {
        self.executions_total.inc();
        self.executions_by_status.with_label_values(&[status]).inc();
        self.execution_duration.with_label_values(&[chain]).observe(duration_seconds);
        
        if let Some(profit) = profit_usd {
            self.execution_profit_usd.observe(profit);
            self.total_profit_usd.inc_by(profit);
        }
        
        if let Some(gas) = gas_used {
            self.execution_gas_used.observe(gas as f64);
        }
    }
    
    /// Record bundle submission
    pub fn record_bundle_submission(&self, relay: &str, status: &str, response_time_seconds: f64) {
        self.bundle_submissions_total.with_label_values(&[relay, status]).inc();
        self.bundle_response_time.with_label_values(&[relay]).observe(response_time_seconds);
    }
    
    /// Record relay error
    pub fn record_relay_error(&self, relay: &str, error_type: &str) {
        self.relay_errors.with_label_values(&[relay, error_type]).inc();
    }
    
    /// Record HTTP request
    pub fn record_http_request(&self, method: &str, endpoint: &str, status: &str, duration_seconds: f64) {
        self.http_requests_total.with_label_values(&[method, endpoint, status]).inc();
        self.http_request_duration.with_label_values(&[method, endpoint]).observe(duration_seconds);
    }
    
    /// Record WebSocket message
    pub fn record_websocket_message(&self, message_type: &str, direction: &str) {
        self.websocket_messages.with_label_values(&[message_type, direction]).inc();
    }
    
    /// Record error
    pub fn record_error(&self, service: &str, error_type: &str, severity: &str) {
        self.errors_total.with_label_values(&[service, error_type, severity]).inc();
    }
    
    /// Update system metrics
    pub fn update_system_metrics(&self, connections: i64, memory_bytes: i64, cpu_percent: f64, db_connections: i64, redis_connections: i64) {
        self.active_connections.set(connections);
        self.memory_usage_bytes.set(memory_bytes);
        self.cpu_usage_percent.set(cpu_percent);
        self.database_connections.set(db_connections);
        self.redis_connections.set(redis_connections);
    }
    
    /// Update business metrics
    pub fn update_business_metrics(&self, net_profit: f64, volume: f64, unique_tokens: i64) {
        self.net_profit_usd.set(net_profit);
        self.arbitrage_volume_usd.inc_by(volume);
        self.unique_tokens_traded.set(unique_tokens);
    }
    
    /// Update success rates
    pub fn update_success_rates(&self, execution_rate: f64, relay_rates: HashMap<String, f64>) {
        self.execution_success_rate.set(execution_rate);
        
        for (relay, rate) in relay_rates {
            self.bundle_success_rate.with_label_values(&[&relay]).set(rate);
        }
    }
    
    /// Update error rates
    pub fn update_error_rates(&self, service_rates: HashMap<String, f64>) {
        for (service, rate) in service_rates {
            self.error_rate.with_label_values(&[&service]).set(rate);
        }
    }
    
    /// Get metrics as Prometheus text format
    pub fn gather(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap_or_default()
    }
    
    /// Reset all metrics (useful for testing)
    pub fn reset(&self) {
        // Note: Prometheus metrics don't have a built-in reset method
        // This would require recreating the metrics or using a test registry
        tracing::warn!("Metrics reset requested - not implemented for production metrics");
    }
}

/// Metrics middleware for HTTP requests
pub struct MetricsMiddleware {
    collector: MetricsCollector,
}

impl MetricsMiddleware {
    pub fn new(collector: MetricsCollector) -> Self {
        Self { collector }
    }
    
    pub fn record_request(&self, method: &str, path: &str, status: u16, duration: std::time::Duration) {
        let status_str = status.to_string();
        let duration_seconds = duration.as_secs_f64();
        
        self.collector.record_http_request(method, path, &status_str, duration_seconds);
    }
}

/// Background metrics collector task
pub async fn start_metrics_collector(collector: MetricsCollector, interval_seconds: u64) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));
    
    loop {
        interval.tick().await;
        
        // Collect system metrics
        if let Some(memory_bytes) = crate::utils::get_memory_usage() {
            collector.memory_usage_bytes.set(memory_bytes as i64);
        }
        
        if let Some(cpu_percent) = crate::utils::get_cpu_usage() {
            collector.cpu_usage_percent.set(cpu_percent);
        }
        
        // Log metrics summary
        tracing::debug!(
            "Metrics collected - Opportunities: {}, Executions: {}, Active connections: {}",
            collector.opportunities_total.get(),
            collector.executions_total.get(),
            collector.active_connections.get()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let collector = MetricsCollector::new();
        
        // Test recording metrics
        collector.record_opportunity("ethereum", "pending", 50.0, 150000);
        assert_eq!(collector.opportunities_total.get(), 1);
        
        collector.record_execution("ethereum", "confirmed", Some(45.0), Some(145000), 15.5);
        assert_eq!(collector.executions_total.get(), 1);
        
        collector.record_bundle_submission("flashbots", "accepted", 0.5);
        collector.record_relay_error("flashbots", "timeout");
        
        collector.record_http_request("GET", "/api/opportunities", "200", 0.025);
        collector.record_websocket_message("opportunity", "outbound");
        collector.record_error("api-server", "validation", "medium");
        
        // Test gathering metrics
        let metrics_text = collector.gather();
        assert!(metrics_text.contains("arbitragex_opportunities_total"));
        assert!(metrics_text.contains("arbitragex_executions_total"));
    }

    #[test]
    fn test_metrics_middleware() {
        let collector = MetricsCollector::new();
        let middleware = MetricsMiddleware::new(collector.clone());
        
        middleware.record_request("GET", "/api/health", 200, std::time::Duration::from_millis(25));
        
        assert_eq!(collector.http_requests_total.with_label_values(&["GET", "/api/health", "200"]).get(), 1);
    }

    #[tokio::test]
    async fn test_system_metrics_update() {
        let collector = MetricsCollector::new();
        
        collector.update_system_metrics(100, 1024 * 1024 * 512, 25.5, 10, 5);
        
        assert_eq!(collector.active_connections.get(), 100);
        assert_eq!(collector.memory_usage_bytes.get(), 1024 * 1024 * 512);
        assert_eq!(collector.cpu_usage_percent.get(), 25.5);
        assert_eq!(collector.database_connections.get(), 10);
        assert_eq!(collector.redis_connections.get(), 5);
    }

    #[test]
    fn test_business_metrics() {
        let collector = MetricsCollector::new();
        
        collector.update_business_metrics(1500.0, 50000.0, 25);
        
        assert_eq!(collector.net_profit_usd.get(), 1500.0);
        assert_eq!(collector.unique_tokens_traded.get(), 25);
    }
}
