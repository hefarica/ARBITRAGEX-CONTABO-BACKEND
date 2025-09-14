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

-- Create opportunities table
CREATE TABLE opportunities (
    id SERIAL PRIMARY KEY,
    chain VARCHAR(50),
    tokens TEXT[],
    profit_usd DECIMAL,
    gas_cost DECIMAL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Create executions table
CREATE TABLE executions (
    id SERIAL PRIMARY KEY,
    opportunity_id INTEGER REFERENCES opportunities(id),
    tx_hash VARCHAR(66),
    success BOOLEAN,
    profit_real DECIMAL,
    executed_at TIMESTAMP DEFAULT NOW()
);

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


