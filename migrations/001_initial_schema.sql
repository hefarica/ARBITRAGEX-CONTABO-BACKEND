-- ArbitrageX Supreme V3.0 - Initial Database Schema Migration
-- Creates all required tables, indexes, and constraints for the ArbitrageX system

-- Enable necessary extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- Create custom types
CREATE TYPE opportunity_status AS ENUM ('pending', 'analyzing', 'selected', 'executing', 'completed', 'failed', 'expired');
CREATE TYPE execution_status AS ENUM ('pending', 'submitted', 'confirmed', 'failed', 'reverted');
CREATE TYPE chain_type AS ENUM ('ethereum', 'polygon', 'arbitrum', 'optimism', 'base', 'bsc');
CREATE TYPE reconciliation_status AS ENUM ('pending', 'in_progress', 'reconciled', 'failed', 'discrepancy');

-- Opportunities table
CREATE TABLE opportunities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    chain chain_type NOT NULL,
    token_a VARCHAR(42) NOT NULL,
    token_b VARCHAR(42) NOT NULL,
    dex_a VARCHAR(100) NOT NULL,
    dex_b VARCHAR(100) NOT NULL,
    amount_in DECIMAL(78, 18) NOT NULL,
    amount_out DECIMAL(78, 18) NOT NULL,
    profit_usd DECIMAL(20, 8) NOT NULL,
    gas_estimate BIGINT NOT NULL,
    gas_price_gwei BIGINT NOT NULL,
    price_impact DECIMAL(10, 6) NOT NULL,
    confidence_score DECIMAL(5, 4) NOT NULL,
    status opportunity_status NOT NULL DEFAULT 'pending',
    block_number BIGINT,
    transaction_hash VARCHAR(66),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Executions table
CREATE TABLE executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    opportunity_id UUID NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    tx_hash VARCHAR(66),
    status execution_status NOT NULL DEFAULT 'pending',
    profit_expected_usd DECIMAL(20, 8),
    profit_actual_usd DECIMAL(20, 8),
    gas_estimate BIGINT,
    gas_used BIGINT,
    gas_price_estimate_gwei BIGINT,
    gas_price_actual_gwei BIGINT,
    block_number BIGINT,
    executed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    confirmed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Reconciliation records table
CREATE TABLE reconciliation_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    execution_id UUID NOT NULL REFERENCES executions(id) ON DELETE CASCADE,
    opportunity_id UUID NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    chain chain_type NOT NULL,
    tx_hash VARCHAR(66),
    block_number BIGINT,
    status reconciliation_status NOT NULL DEFAULT 'pending',
    expected_profit_usd DECIMAL(20, 8),
    actual_profit_usd DECIMAL(20, 8),
    gas_estimate BIGINT,
    gas_used BIGINT,
    gas_price_gwei BIGINT,
    discrepancies JSONB DEFAULT '[]'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    reconciled_at TIMESTAMP WITH TIME ZONE
);

-- Bundle submissions table
CREATE TABLE bundle_submissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bundle_id VARCHAR(100) NOT NULL,
    relay_name VARCHAR(50) NOT NULL,
    execution_id UUID REFERENCES executions(id) ON DELETE CASCADE,
    transactions JSONB NOT NULL,
    block_number BIGINT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    submission_time TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    response JSONB,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0
);

-- System metrics table
CREATE TABLE system_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_name VARCHAR(100) NOT NULL,
    metric_value DECIMAL(20, 8) NOT NULL,
    metric_type VARCHAR(20) NOT NULL, -- 'counter', 'gauge', 'histogram'
    labels JSONB DEFAULT '{}'::jsonb,
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User sessions table
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,
    session_token VARCHAR(500) NOT NULL UNIQUE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_activity TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT
);

-- API keys table
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_name VARCHAR(100) NOT NULL,
    key_hash VARCHAR(128) NOT NULL UNIQUE,
    permissions JSONB DEFAULT '[]'::jsonb,
    rate_limit_per_minute INTEGER DEFAULT 100,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE
);

-- Configuration table
CREATE TABLE configuration (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    config_key VARCHAR(100) NOT NULL UNIQUE,
    config_value JSONB NOT NULL,
    description TEXT,
    is_encrypted BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Audit log table
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID NOT NULL,
    action VARCHAR(20) NOT NULL, -- 'create', 'update', 'delete'
    old_values JSONB,
    new_values JSONB,
    user_id VARCHAR(100),
    ip_address INET,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_opportunities_chain ON opportunities(chain);
CREATE INDEX idx_opportunities_status ON opportunities(status);
CREATE INDEX idx_opportunities_created_at ON opportunities(created_at);
CREATE INDEX idx_opportunities_profit_usd ON opportunities(profit_usd DESC);
CREATE INDEX idx_opportunities_expires_at ON opportunities(expires_at);
CREATE INDEX idx_opportunities_block_number ON opportunities(block_number);

CREATE INDEX idx_executions_opportunity_id ON executions(opportunity_id);
CREATE INDEX idx_executions_status ON executions(status);
CREATE INDEX idx_executions_executed_at ON executions(executed_at);
CREATE INDEX idx_executions_tx_hash ON executions(tx_hash);
CREATE INDEX idx_executions_block_number ON executions(block_number);

CREATE INDEX idx_reconciliation_execution_id ON reconciliation_records(execution_id);
CREATE INDEX idx_reconciliation_status ON reconciliation_records(status);
CREATE INDEX idx_reconciliation_created_at ON reconciliation_records(created_at);

CREATE INDEX idx_bundle_submissions_bundle_id ON bundle_submissions(bundle_id);
CREATE INDEX idx_bundle_submissions_relay_name ON bundle_submissions(relay_name);
CREATE INDEX idx_bundle_submissions_status ON bundle_submissions(status);
CREATE INDEX idx_bundle_submissions_submission_time ON bundle_submissions(submission_time);

CREATE INDEX idx_system_metrics_name ON system_metrics(metric_name);
CREATE INDEX idx_system_metrics_recorded_at ON system_metrics(recorded_at);
CREATE INDEX idx_system_metrics_labels ON system_metrics USING GIN(labels);

CREATE INDEX idx_user_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_user_sessions_expires_at ON user_sessions(expires_at);
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);

CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_active ON api_keys(is_active);

CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);

-- Composite indexes for common queries
CREATE INDEX idx_opportunities_chain_status ON opportunities(chain, status);
CREATE INDEX idx_opportunities_status_created ON opportunities(status, created_at);
CREATE INDEX idx_executions_status_executed ON executions(status, executed_at);

-- Partial indexes for active records
CREATE INDEX idx_opportunities_active ON opportunities(created_at) WHERE status IN ('pending', 'analyzing', 'selected');
CREATE INDEX idx_executions_active ON executions(executed_at) WHERE status IN ('pending', 'submitted');

-- GIN indexes for JSONB columns
CREATE INDEX idx_opportunities_metadata ON opportunities USING GIN(metadata);
CREATE INDEX idx_executions_metadata ON executions USING GIN(metadata);
CREATE INDEX idx_reconciliation_discrepancies ON reconciliation_records USING GIN(discrepancies);

-- Functions and triggers for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_opportunities_updated_at BEFORE UPDATE ON opportunities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_configuration_updated_at BEFORE UPDATE ON configuration
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to clean up expired opportunities
CREATE OR REPLACE FUNCTION cleanup_expired_opportunities()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM opportunities 
    WHERE expires_at < NOW() AND status IN ('pending', 'analyzing');
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to calculate profit metrics
CREATE OR REPLACE FUNCTION calculate_profit_metrics(
    start_date TIMESTAMP WITH TIME ZONE DEFAULT NOW() - INTERVAL '24 hours',
    end_date TIMESTAMP WITH TIME ZONE DEFAULT NOW()
)
RETURNS TABLE(
    total_profit_usd DECIMAL(20, 8),
    total_gas_cost_usd DECIMAL(20, 8),
    net_profit_usd DECIMAL(20, 8),
    execution_count BIGINT,
    success_rate DECIMAL(5, 4)
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        COALESCE(SUM(e.profit_actual_usd), 0) as total_profit_usd,
        COALESCE(SUM(e.gas_used * e.gas_price_actual_gwei / 1e9 * 2000), 0) as total_gas_cost_usd, -- Assuming $2000 ETH
        COALESCE(SUM(e.profit_actual_usd), 0) - COALESCE(SUM(e.gas_used * e.gas_price_actual_gwei / 1e9 * 2000), 0) as net_profit_usd,
        COUNT(*) as execution_count,
        ROUND(COUNT(*) FILTER (WHERE e.status = 'confirmed')::DECIMAL / NULLIF(COUNT(*), 0), 4) as success_rate
    FROM executions e
    WHERE e.executed_at BETWEEN start_date AND end_date;
END;
$$ LANGUAGE plpgsql;

-- Views for common queries
CREATE VIEW v_active_opportunities AS
SELECT 
    o.*,
    CASE 
        WHEN o.expires_at < NOW() THEN true 
        ELSE false 
    END as is_expired
FROM opportunities o
WHERE o.status IN ('pending', 'analyzing', 'selected')
ORDER BY o.profit_usd DESC;

CREATE VIEW v_execution_summary AS
SELECT 
    e.*,
    o.chain,
    o.token_a,
    o.token_b,
    o.dex_a,
    o.dex_b,
    o.profit_usd as expected_profit_usd
FROM executions e
JOIN opportunities o ON e.opportunity_id = o.id
ORDER BY e.executed_at DESC;

CREATE VIEW v_daily_metrics AS
SELECT 
    DATE(executed_at) as date,
    COUNT(*) as total_executions,
    COUNT(*) FILTER (WHERE status = 'confirmed') as successful_executions,
    SUM(profit_actual_usd) FILTER (WHERE status = 'confirmed') as total_profit_usd,
    AVG(profit_actual_usd) FILTER (WHERE status = 'confirmed') as avg_profit_usd,
    SUM(gas_used * gas_price_actual_gwei / 1e9) FILTER (WHERE status = 'confirmed') as total_gas_cost_eth
FROM executions
WHERE executed_at >= NOW() - INTERVAL '30 days'
GROUP BY DATE(executed_at)
ORDER BY date DESC;

-- Insert default configuration
INSERT INTO configuration (config_key, config_value, description) VALUES
('min_profit_usd', '10.0', 'Minimum profit threshold in USD for opportunity execution'),
('max_gas_price_gwei', '100', 'Maximum gas price in Gwei for transaction submission'),
('max_slippage_percent', '2.0', 'Maximum allowed slippage percentage'),
('execution_timeout_seconds', '30', 'Timeout for execution operations'),
('confirmation_blocks', '1', 'Number of blocks to wait for confirmation'),
('retry_attempts', '3', 'Number of retry attempts for failed operations'),
('rate_limit_per_minute', '100', 'Default rate limit per minute for API calls'),
('session_timeout_hours', '24', 'Session timeout in hours'),
('cleanup_interval_hours', '6', 'Interval for cleanup operations in hours'),
('metrics_retention_days', '30', 'Number of days to retain metrics data');

-- Grant permissions (adjust as needed for your setup)
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO arbitragex;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO arbitragex;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO arbitragex;

-- Create a user for read-only access (monitoring, analytics)
-- CREATE USER arbitragex_readonly WITH PASSWORD 'readonly_password';
-- GRANT CONNECT ON DATABASE arbitragex_supreme TO arbitragex_readonly;
-- GRANT USAGE ON SCHEMA public TO arbitragex_readonly;
-- GRANT SELECT ON ALL TABLES IN SCHEMA public TO arbitragex_readonly;

COMMENT ON DATABASE arbitragex_supreme IS 'ArbitrageX Supreme V3.0 - Main application database';
COMMENT ON TABLE opportunities IS 'Detected arbitrage opportunities across different chains and DEXs';
COMMENT ON TABLE executions IS 'Execution attempts for arbitrage opportunities';
COMMENT ON TABLE reconciliation_records IS 'Reconciliation data for executed transactions';
COMMENT ON TABLE bundle_submissions IS 'MEV bundle submissions to various relays';
COMMENT ON TABLE system_metrics IS 'System performance and business metrics';
COMMENT ON TABLE user_sessions IS 'Active user sessions for authentication';
COMMENT ON TABLE api_keys IS 'API keys for external access';
COMMENT ON TABLE configuration IS 'System configuration parameters';
COMMENT ON TABLE audit_log IS 'Audit trail for all system changes';
