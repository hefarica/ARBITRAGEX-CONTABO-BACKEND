-- ArbitrageX Supreme v3.0 - Database Schema
-- PostgreSQL 15+ required

-- Create custom types
CREATE TYPE opportunity_status AS ENUM (
    'pending',
    'simulating', 
    'approved',
    'executing',
    'completed',
    'failed',
    'expired'
);

CREATE TYPE execution_status AS ENUM (
    'pending',
    'submitted',
    'confirmed', 
    'failed',
    'reverted'
);

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- Opportunities table with comprehensive tracking
CREATE TABLE opportunities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    opportunity_type VARCHAR(50) NOT NULL,
    chain VARCHAR(50) NOT NULL,
    tokens JSONB NOT NULL,
    dexes JSONB NOT NULL,
    profit_estimate_usd DECIMAL(18,6) NOT NULL,
    gas_estimate BIGINT NOT NULL,
    gas_price_gwei BIGINT NOT NULL,
    confidence_score DECIMAL(3,2) NOT NULL,
    risk_score DECIMAL(3,2) NOT NULL,
    transaction_data JSONB,
    status VARCHAR(20) DEFAULT 'detected',
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Executions table with detailed tracking
CREATE TABLE executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    opportunity_id UUID REFERENCES opportunities(id) ON DELETE CASCADE,
    tx_hash VARCHAR(66) UNIQUE,
    bundle_hash VARCHAR(66),
    relay_name VARCHAR(50),
    status VARCHAR(20) DEFAULT 'pending',
    success BOOLEAN,
    profit_actual_usd DECIMAL(18,6),
    gas_used BIGINT,
    gas_price_actual_gwei BIGINT,
    execution_time_ms INTEGER,
    error_message TEXT,
    block_number BIGINT,
    transaction_index INTEGER,
    executed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    confirmed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Strategies table for tracking different MEV strategies
CREATE TABLE strategies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE,
    strategy_type VARCHAR(50) NOT NULL,
    parameters JSONB NOT NULL,
    is_active BOOLEAN DEFAULT true,
    min_profit_threshold_usd DECIMAL(18,6) DEFAULT 10.0,
    max_gas_price_gwei BIGINT DEFAULT 100,
    success_rate DECIMAL(5,4) DEFAULT 0.0,
    total_executions INTEGER DEFAULT 0,
    total_profit_usd DECIMAL(18,6) DEFAULT 0.0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Market data table for price tracking
CREATE TABLE market_data (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    chain VARCHAR(50) NOT NULL,
    token_address VARCHAR(42) NOT NULL,
    token_symbol VARCHAR(20) NOT NULL,
    price_usd DECIMAL(18,8) NOT NULL,
    volume_24h_usd DECIMAL(18,2),
    market_cap_usd DECIMAL(18,2),
    source VARCHAR(50) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain, token_address, source, timestamp)
);

-- DEX pools table for liquidity tracking
CREATE TABLE dex_pools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    chain VARCHAR(50) NOT NULL,
    dex_name VARCHAR(50) NOT NULL,
    pool_address VARCHAR(42) NOT NULL,
    token0_address VARCHAR(42) NOT NULL,
    token0_symbol VARCHAR(20) NOT NULL,
    token1_address VARCHAR(42) NOT NULL,
    token1_symbol VARCHAR(20) NOT NULL,
    fee_bps INTEGER NOT NULL,
    tvl_usd DECIMAL(18,2),
    volume_24h_usd DECIMAL(18,2),
    reserve0 DECIMAL(36,18),
    reserve1 DECIMAL(36,18),
    is_active BOOLEAN DEFAULT true,
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain, dex_name, pool_address)
);

-- Gas prices table for tracking gas costs
CREATE TABLE gas_prices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    chain VARCHAR(50) NOT NULL,
    base_fee_gwei DECIMAL(10,2),
    priority_fee_gwei DECIMAL(10,2),
    fast_gas_price_gwei DECIMAL(10,2),
    standard_gas_price_gwei DECIMAL(10,2),
    safe_gas_price_gwei DECIMAL(10,2),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Analytics table for performance metrics
CREATE TABLE analytics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_name VARCHAR(100) NOT NULL,
    metric_value DECIMAL(18,6) NOT NULL,
    metric_unit VARCHAR(20),
    chain VARCHAR(50),
    strategy_id UUID REFERENCES strategies(id),
    time_period VARCHAR(20) NOT NULL, -- '1h', '1d', '7d', '30d'
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- System health table for monitoring
CREATE TABLE system_health (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL, -- 'healthy', 'degraded', 'down'
    response_time_ms INTEGER,
    error_rate DECIMAL(5,4),
    last_error TEXT,
    metadata JSONB,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User sessions table for frontend auth
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_token VARCHAR(255) NOT NULL UNIQUE,
    user_id VARCHAR(100) NOT NULL,
    ip_address INET,
    user_agent TEXT,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Temporal workflows table for orchestration tracking
CREATE TABLE temporal_workflows (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workflow_id VARCHAR(255) NOT NULL UNIQUE,
    workflow_type VARCHAR(100) NOT NULL,
    run_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    opportunity_id UUID REFERENCES opportunities(id),
    execution_id UUID REFERENCES executions(id),
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    result JSONB
);

-- Activepieces flows table for automation tracking
CREATE TABLE activepieces_flows (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    flow_id VARCHAR(255) NOT NULL,
    flow_name VARCHAR(255) NOT NULL,
    trigger_type VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL,
    execution_count INTEGER DEFAULT 0,
    last_execution TIMESTAMP WITH TIME ZONE,
    configuration JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_opportunities_chain_status ON opportunities(chain, status);
CREATE INDEX idx_opportunities_created_at ON opportunities(created_at DESC);
CREATE INDEX idx_opportunities_profit ON opportunities(profit_estimate_usd DESC);
CREATE INDEX idx_opportunities_expires_at ON opportunities(expires_at);
CREATE INDEX idx_opportunities_type ON opportunities(opportunity_type);

CREATE INDEX idx_executions_opportunity_id ON executions(opportunity_id);
CREATE INDEX idx_executions_tx_hash ON executions(tx_hash);
CREATE INDEX idx_executions_status ON executions(status);
CREATE INDEX idx_executions_executed_at ON executions(executed_at DESC);
CREATE INDEX idx_executions_success ON executions(success);

CREATE INDEX idx_market_data_chain_token ON market_data(chain, token_address);
CREATE INDEX idx_market_data_timestamp ON market_data(timestamp DESC);
CREATE INDEX idx_market_data_symbol ON market_data(token_symbol);

CREATE INDEX idx_dex_pools_chain_dex ON dex_pools(chain, dex_name);
CREATE INDEX idx_dex_pools_tokens ON dex_pools(token0_address, token1_address);
CREATE INDEX idx_dex_pools_tvl ON dex_pools(tvl_usd DESC);
CREATE INDEX idx_dex_pools_active ON dex_pools(is_active);

CREATE INDEX idx_gas_prices_chain_timestamp ON gas_prices(chain, timestamp DESC);

CREATE INDEX idx_analytics_metric_time ON analytics(metric_name, time_period, timestamp DESC);
CREATE INDEX idx_analytics_strategy ON analytics(strategy_id, timestamp DESC);

CREATE INDEX idx_system_health_service_timestamp ON system_health(service_name, timestamp DESC);

CREATE INDEX idx_user_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_user_sessions_expires ON user_sessions(expires_at);

CREATE INDEX idx_temporal_workflows_id ON temporal_workflows(workflow_id);
CREATE INDEX idx_temporal_workflows_status ON temporal_workflows(status);
CREATE INDEX idx_temporal_workflows_opportunity ON temporal_workflows(opportunity_id);

-- Create triggers for updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_opportunities_updated_at BEFORE UPDATE ON opportunities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_strategies_updated_at BEFORE UPDATE ON strategies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create views for common queries
CREATE VIEW v_opportunity_summary AS
SELECT 
    o.id,
    o.opportunity_type,
    o.chain,
    o.profit_estimate_usd,
    o.confidence_score,
    o.status,
    o.created_at,
    e.status as execution_status,
    e.profit_actual_usd,
    e.tx_hash
FROM opportunities o
LEFT JOIN executions e ON o.id = e.opportunity_id;

CREATE VIEW v_strategy_performance AS
SELECT 
    s.name,
    s.strategy_type,
    s.is_active,
    s.total_executions,
    s.total_profit_usd,
    s.success_rate,
    COUNT(o.id) as opportunities_detected,
    COUNT(e.id) as executions_attempted,
    COUNT(CASE WHEN e.success = true THEN 1 END) as successful_executions,
    AVG(e.profit_actual_usd) as avg_profit_per_execution
FROM strategies s
LEFT JOIN opportunities o ON o.opportunity_type = s.strategy_type
LEFT JOIN executions e ON o.id = e.opportunity_id
GROUP BY s.id, s.name, s.strategy_type, s.is_active, s.total_executions, s.total_profit_usd, s.success_rate;

CREATE VIEW v_daily_performance AS
SELECT 
    DATE(executed_at) as execution_date,
    COUNT(*) as total_executions,
    COUNT(CASE WHEN success = true THEN 1 END) as successful_executions,
    SUM(profit_actual_usd) as total_profit_usd,
    AVG(profit_actual_usd) as avg_profit_usd,
    AVG(gas_used) as avg_gas_used,
    AVG(execution_time_ms) as avg_execution_time_ms
FROM executions
WHERE executed_at >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE(executed_at)
ORDER BY execution_date DESC;

-- Insert default strategies
INSERT INTO strategies (name, strategy_type, parameters, min_profit_threshold_usd, max_gas_price_gwei) VALUES
('Triangular Arbitrage ETH', 'TriangularArbitrage', '{"base_token": "WETH", "min_liquidity": 100000}', 25.0, 80),
('Flash Loan Arbitrage', 'FlashLoanArbitrage', '{"max_loan_amount": 1000000, "min_price_diff": 0.005}', 50.0, 100),
('Cross-DEX Arbitrage', 'CrossDexArbitrage', '{"dexes": ["uniswap_v2", "sushiswap"], "min_tvl": 50000}', 30.0, 90),
('Sandwich Attack', 'SandwichAttack', '{"min_trade_size": 10000, "max_slippage": 0.03}', 15.0, 120),
('Liquidation Bot', 'Liquidation', '{"protocols": ["aave", "compound"], "health_factor_threshold": 1.05}', 40.0, 150);

-- Insert system health monitoring entries
INSERT INTO system_health (service_name, status, response_time_ms, error_rate) VALUES
('searcher-rs', 'healthy', 50, 0.0),
('selector-api', 'healthy', 100, 0.0),
('sim-ctl', 'healthy', 200, 0.0),
('relays-client', 'healthy', 300, 0.0),
('recon', 'healthy', 150, 0.0),
('temporal-server', 'healthy', 80, 0.0),
('activepieces', 'healthy', 120, 0.0),
('postgres-db', 'healthy', 20, 0.0),
('redis-cache', 'healthy', 10, 0.0),
('geth-node', 'healthy', 500, 0.0);

-- Grant permissions
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO arbitragex;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO arbitragex;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO arbitragex;

-- Create read-only user for analytics
CREATE USER arbitragex_readonly WITH PASSWORD 'readonly_password';
GRANT CONNECT ON DATABASE arbitragex TO arbitragex_readonly;
GRANT USAGE ON SCHEMA public TO arbitragex_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO arbitragex_readonly;

-- Comments for documentation
COMMENT ON TABLE opportunities IS 'Detected MEV opportunities with comprehensive metadata';
COMMENT ON TABLE executions IS 'Execution attempts and results for opportunities';
COMMENT ON TABLE strategies IS 'MEV strategy configurations and performance metrics';
COMMENT ON TABLE market_data IS 'Real-time token price and market data';
COMMENT ON TABLE dex_pools IS 'DEX pool information and liquidity data';
COMMENT ON TABLE gas_prices IS 'Historical gas price data for cost optimization';
COMMENT ON TABLE analytics IS 'Performance metrics and KPIs';
COMMENT ON TABLE system_health IS 'System component health monitoring';
COMMENT ON TABLE temporal_workflows IS 'Temporal.io workflow execution tracking';
COMMENT ON TABLE activepieces_flows IS 'Activepieces automation flow tracking';

-- Create pools table for caching
CREATE TABLE IF NOT EXISTS pools (
    address VARCHAR(42) PRIMARY KEY,
    token0 VARCHAR(42) NOT NULL,
    token1 VARCHAR(42) NOT NULL,
    reserve0 VARCHAR(78) NOT NULL,
    reserve1 VARCHAR(78) NOT NULL,
    fee INTEGER NOT NULL,
    tvl_usd NUMERIC(20, 2),
    active BOOLEAN DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create strategies table
CREATE TABLE IF NOT EXISTS strategies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    enabled BOOLEAN DEFAULT true,
    min_profit_wei BIGINT NOT NULL DEFAULT 10000000000000000, -- 0.01 ETH
    max_gas_price BIGINT NOT NULL DEFAULT 100000000000, -- 100 gwei
    config JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create gas_prices table
CREATE TABLE IF NOT EXISTS gas_prices (
    id SERIAL PRIMARY KEY,
    block_number BIGINT NOT NULL,
    base_fee BIGINT NOT NULL,
    priority_fee BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create pnl_history table
CREATE TABLE IF NOT EXISTS pnl_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    date DATE NOT NULL,
    total_opportunities INTEGER NOT NULL DEFAULT 0,
    successful_executions INTEGER NOT NULL DEFAULT 0,
    failed_executions INTEGER NOT NULL DEFAULT 0,
    total_profit BIGINT NOT NULL DEFAULT 0,
    total_gas_cost BIGINT NOT NULL DEFAULT 0,
    net_profit BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create api_keys table
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    permissions JSONB NOT NULL DEFAULT '{"read": true, "write": false, "execute": false}',
    rate_limit INTEGER NOT NULL DEFAULT 100,
    enabled BOOLEAN DEFAULT true,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

-- Create indexes
CREATE INDEX idx_pools_active ON pools(active);
CREATE INDEX idx_pools_tokens ON pools(token0, token1);
CREATE INDEX idx_gas_prices_block ON gas_prices(block_number DESC);
CREATE INDEX idx_pnl_history_date ON pnl_history(date DESC);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);

-- Create update trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply trigger to tables with updated_at
CREATE TRIGGER update_pools_updated_at BEFORE UPDATE
    ON pools FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_strategies_updated_at BEFORE UPDATE
    ON strategies FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default strategies
INSERT INTO strategies (name, description, enabled, min_profit_wei, max_gas_price, config) VALUES
('arbitrage', 'Multi-DEX arbitrage strategy', true, 10000000000000000, 100000000000, '{"max_hops": 3, "dexes": ["uniswap", "sushiswap", "balancer"]}'),
('sandwich', 'MEV sandwich attack strategy', true, 50000000000000000, 150000000000, '{"min_victim_amount": "1000000000000000000"}'),
('liquidation', 'DeFi lending liquidation strategy', true, 100000000000000000, 200000000000, '{"protocols": ["aave", "compound", "maker"]}')
ON CONFLICT (name) DO NOTHING;

-- Create materialized view for performance metrics
CREATE MATERIALIZED VIEW IF NOT EXISTS daily_metrics AS
SELECT 
    date_trunc('day', created_at) as date,
    COUNT(*) as total_opportunities,
    COUNT(*) FILTER (WHERE status = 'completed') as successful,
    COUNT(*) FILTER (WHERE status = 'failed') as failed,
    SUM(expected_profit) FILTER (WHERE status = 'completed') as total_profit,
    AVG(gas_cost) as avg_gas_cost,
    MAX(expected_profit) as max_profit
FROM opportunities
GROUP BY date_trunc('day', created_at)
ORDER BY date DESC;

-- Create index on materialized view
CREATE INDEX idx_daily_metrics_date ON daily_metrics(date);

-- Refresh materialized view function
CREATE OR REPLACE FUNCTION refresh_daily_metrics()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY daily_metrics;
END;
$$ LANGUAGE plpgsql;


