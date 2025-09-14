-- ArbitrageX Supreme V3.0 - Database Initialization Script
-- PostgreSQL database schema for MEV arbitrage system
-- Version: 3.0.0

-- Set timezone
SET timezone = 'UTC';

-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- Create custom types
CREATE TYPE strategy_type AS ENUM (
    'SimpleArbitrage',
    'TriangularArbitrage', 
    'StatisticalArbitrage',
    'JITLiquidityBackrun',
    'MultiDexArbitrage',
    'CrossChainArbitrage',
    'FlashLoanArbitrage',
    'LiquidationMEV',
    'SandwichAttack',
    'NFTArbitrage',
    'StatisticalZScore',
    'GovernanceArbitrage',
    'OracleMEV',
    'YieldFarmingArbitrage',
    'OptionsArbitrage',
    'MultiDexTriangular',
    'CrossChainBridge',
    'NFTCrossMarketplace',
    'DeFiComposite',
    'GovernanceMEVProtection'
);

CREATE TYPE opportunity_status AS ENUM (
    'detected',
    'validated',
    'executing',
    'executed',
    'failed',
    'expired'
);

CREATE TYPE execution_status AS ENUM (
    'pending',
    'submitted',
    'included',
    'failed',
    'reverted'
);

CREATE TYPE competition_level AS ENUM (
    'low',
    'medium', 
    'high',
    'extreme'
);

-- Main tables
-- Arbitrage opportunities table
CREATE TABLE opportunities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    strategy_type strategy_type NOT NULL,
    token_in VARCHAR(42) NOT NULL,
    token_out VARCHAR(42) NOT NULL,
    amount_in NUMERIC(78, 0) NOT NULL,
    amount_out NUMERIC(78, 0) NOT NULL,
    estimated_profit DECIMAL(20, 8) NOT NULL,
    estimated_gas BIGINT NOT NULL,
    net_profit DECIMAL(20, 8) NOT NULL,
    source_dex VARCHAR(50) NOT NULL,
    target_dex VARCHAR(50) NOT NULL,
    price_impact DECIMAL(10, 4) NOT NULL,
    slippage_tolerance DECIMAL(10, 4) NOT NULL,
    requires_flash_loan BOOLEAN NOT NULL DEFAULT FALSE,
    flash_loan_amount NUMERIC(78, 0),
    route TEXT[], -- Array of addresses
    pools TEXT[], -- Array of pool addresses
    block_number BIGINT NOT NULL,
    detected_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    priority_score DECIMAL(10, 4) NOT NULL,
    competition_level competition_level NOT NULL DEFAULT 'medium',
    status opportunity_status NOT NULL DEFAULT 'detected',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Executions table
CREATE TABLE executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    opportunity_id UUID NOT NULL REFERENCES opportunities(id),
    tx_hash VARCHAR(66) UNIQUE,
    bundle_hash VARCHAR(66),
    relay_name VARCHAR(50),
    gas_price BIGINT NOT NULL,
    gas_limit BIGINT NOT NULL,
    gas_used BIGINT,
    actual_profit DECIMAL(20, 8),
    execution_time_ms INTEGER,
    block_number BIGINT,
    transaction_index INTEGER,
    status execution_status NOT NULL DEFAULT 'pending',
    error_message TEXT,
    simulation_result JSONB,
    submitted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    executed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Flash loans table
CREATE TABLE flash_loans (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    execution_id UUID NOT NULL REFERENCES executions(id),
    provider_name VARCHAR(50) NOT NULL,
    provider_address VARCHAR(42) NOT NULL,
    token_address VARCHAR(42) NOT NULL,
    amount NUMERIC(78, 0) NOT NULL,
    fee NUMERIC(78, 0) NOT NULL,
    fee_rate DECIMAL(10, 6) NOT NULL,
    tx_hash VARCHAR(66),
    status execution_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Market data table
CREATE TABLE market_data (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token_address VARCHAR(42) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    decimals SMALLINT NOT NULL,
    price_usd DECIMAL(20, 8) NOT NULL,
    volume_24h DECIMAL(20, 8) NOT NULL,
    liquidity_usd DECIMAL(20, 8) NOT NULL,
    price_change_24h DECIMAL(10, 4) NOT NULL,
    block_number BIGINT NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Price quotes table
CREATE TABLE price_quotes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dex_name VARCHAR(50) NOT NULL,
    token_in VARCHAR(42) NOT NULL,
    token_out VARCHAR(42) NOT NULL,
    amount_in NUMERIC(78, 0) NOT NULL,
    amount_out NUMERIC(78, 0) NOT NULL,
    price DECIMAL(30, 18) NOT NULL,
    fee INTEGER NOT NULL,
    slippage DECIMAL(10, 4) NOT NULL,
    gas_estimate BIGINT NOT NULL,
    pool_address VARCHAR(42) NOT NULL,
    block_number BIGINT NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- DEX information table
CREATE TABLE dex_info (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(50) UNIQUE NOT NULL,
    router_address VARCHAR(42) NOT NULL,
    factory_address VARCHAR(42) NOT NULL,
    fee_tier INTEGER NOT NULL,
    supports_flash_swap BOOLEAN NOT NULL DEFAULT FALSE,
    gas_overhead BIGINT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Token pairs table
CREATE TABLE token_pairs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token0 VARCHAR(42) NOT NULL,
    token1 VARCHAR(42) NOT NULL,
    pool_address VARCHAR(42) UNIQUE NOT NULL,
    dex_name VARCHAR(50) NOT NULL,
    fee INTEGER NOT NULL,
    liquidity NUMERIC(78, 0) NOT NULL,
    price DECIMAL(30, 18) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Performance metrics table
CREATE TABLE performance_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_name VARCHAR(100) NOT NULL,
    metric_value DECIMAL(20, 8) NOT NULL,
    metric_type VARCHAR(50) NOT NULL, -- 'counter', 'gauge', 'histogram'
    labels JSONB DEFAULT '{}',
    block_number BIGINT,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- System logs table
CREATE TABLE system_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    level VARCHAR(10) NOT NULL, -- 'DEBUG', 'INFO', 'WARN', 'ERROR', 'FATAL'
    message TEXT NOT NULL,
    module VARCHAR(50) NOT NULL,
    trace_id UUID,
    span_id VARCHAR(32),
    metadata JSONB DEFAULT '{}',
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for performance optimization
-- Opportunities indexes
CREATE INDEX idx_opportunities_strategy_type ON opportunities(strategy_type);
CREATE INDEX idx_opportunities_status ON opportunities(status);
CREATE INDEX idx_opportunities_detected_at ON opportunities(detected_at);
CREATE INDEX idx_opportunities_block_number ON opportunities(block_number);
CREATE INDEX idx_opportunities_expires_at ON opportunities(expires_at);
CREATE INDEX idx_opportunities_net_profit ON opportunities(net_profit DESC);
CREATE INDEX idx_opportunities_priority_score ON opportunities(priority_score DESC);

-- Executions indexes
CREATE INDEX idx_executions_opportunity_id ON executions(opportunity_id);
CREATE INDEX idx_executions_tx_hash ON executions(tx_hash);
CREATE INDEX idx_executions_status ON executions(status);
CREATE INDEX idx_executions_submitted_at ON executions(submitted_at);
CREATE INDEX idx_executions_block_number ON executions(block_number);

-- Market data indexes
CREATE INDEX idx_market_data_token_address ON market_data(token_address);
CREATE INDEX idx_market_data_timestamp ON market_data(timestamp);
CREATE INDEX idx_market_data_block_number ON market_data(block_number);

-- Price quotes indexes
CREATE INDEX idx_price_quotes_dex_name ON price_quotes(dex_name);
CREATE INDEX idx_price_quotes_tokens ON price_quotes(token_in, token_out);
CREATE INDEX idx_price_quotes_timestamp ON price_quotes(timestamp);
CREATE INDEX idx_price_quotes_block_number ON price_quotes(block_number);

-- Token pairs indexes
CREATE INDEX idx_token_pairs_tokens ON token_pairs(token0, token1);
CREATE INDEX idx_token_pairs_pool_address ON token_pairs(pool_address);
CREATE INDEX idx_token_pairs_dex_name ON token_pairs(dex_name);
CREATE INDEX idx_token_pairs_is_active ON token_pairs(is_active);

-- Performance metrics indexes
CREATE INDEX idx_performance_metrics_name ON performance_metrics(metric_name);
CREATE INDEX idx_performance_metrics_timestamp ON performance_metrics(timestamp);
CREATE INDEX idx_performance_metrics_labels ON performance_metrics USING GIN(labels);

-- System logs indexes
CREATE INDEX idx_system_logs_level ON system_logs(level);
CREATE INDEX idx_system_logs_module ON system_logs(module);
CREATE INDEX idx_system_logs_timestamp ON system_logs(timestamp);
CREATE INDEX idx_system_logs_trace_id ON system_logs(trace_id);

-- Composite indexes for complex queries
CREATE INDEX idx_opportunities_composite ON opportunities(status, strategy_type, detected_at);
CREATE INDEX idx_executions_composite ON executions(status, submitted_at, opportunity_id);
CREATE INDEX idx_price_quotes_composite ON price_quotes(dex_name, token_in, token_out, timestamp);

-- Partitioning for large tables (optional, can be enabled later)
-- ALTER TABLE price_quotes PARTITION BY RANGE (timestamp);
-- ALTER TABLE performance_metrics PARTITION BY RANGE (timestamp);
-- ALTER TABLE system_logs PARTITION BY RANGE (timestamp);

-- Update timestamp triggers
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply update triggers
CREATE TRIGGER update_opportunities_updated_at BEFORE UPDATE ON opportunities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_executions_updated_at BEFORE UPDATE ON executions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_dex_info_updated_at BEFORE UPDATE ON dex_info
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default DEX information
INSERT INTO dex_info (name, router_address, factory_address, fee_tier, supports_flash_swap, gas_overhead) VALUES
('Uniswap V2', '0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D', '0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f', 300, true, 150000),
('Uniswap V3', '0xE592427A0AEce92De3Edee1F18E0157C05861564', '0x1F98431c8aD98523631AE4a59f267346ea31F984', 500, true, 180000),
('SushiSwap', '0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F', '0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac', 300, true, 160000),
('Balancer V2', '0xBA12222222228d8Ba445958a75a0704d566BF2C8', '0xBA12222222228d8Ba445958a75a0704d566BF2C8', 100, false, 200000),
('Curve', '0x99a58482BD75cbab83b27EC03CA68fF489b5788f', '0x0959158b6040D32d04c301A72CBFD6b39E21c9AE', 400, false, 250000);

-- Create views for common queries
CREATE VIEW v_recent_opportunities AS
SELECT 
    o.*,
    e.status as execution_status,
    e.tx_hash,
    e.actual_profit
FROM opportunities o
LEFT JOIN executions e ON o.id = e.opportunity_id
WHERE o.detected_at >= NOW() - INTERVAL '1 hour'
ORDER BY o.detected_at DESC;

CREATE VIEW v_performance_summary AS
SELECT 
    DATE(o.detected_at) as date,
    o.strategy_type,
    COUNT(*) as total_opportunities,
    COUNT(CASE WHEN e.status = 'included' THEN 1 END) as successful_executions,
    SUM(CASE WHEN e.status = 'included' THEN e.actual_profit ELSE 0 END) as total_profit,
    AVG(CASE WHEN e.status = 'included' THEN e.execution_time_ms END) as avg_execution_time
FROM opportunities o
LEFT JOIN executions e ON o.id = e.opportunity_id
WHERE o.detected_at >= NOW() - INTERVAL '30 days'
GROUP BY DATE(o.detected_at), o.strategy_type
ORDER BY date DESC, total_profit DESC;

-- Grant permissions
GRANT USAGE ON SCHEMA public TO arbitragex;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO arbitragex;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO arbitragex;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO arbitragex;