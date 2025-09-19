-- ArbitrageX Supreme V3.0 - Database Migrations
-- Paquete Operativo Completo - PostgreSQL & D1

-- =====================================================
-- POSTGRESQL MIGRATIONS (Backend/Contabo)
-- Ubicación: migrations/001_initial_schema.sql
-- =====================================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- Create custom types
CREATE TYPE opportunity_status AS ENUM ('active', 'executed', 'expired', 'failed');
CREATE TYPE execution_status AS ENUM ('pending', 'executing', 'completed', 'failed', 'cancelled');
CREATE TYPE strategy_type AS ENUM ('arbitrage', 'flashloan', 'mev', 'cross_chain');
CREATE TYPE chain_type AS ENUM ('ethereum', 'polygon', 'bsc', 'arbitrum', 'optimism');

-- =====================================================
-- CORE TABLES
-- =====================================================

-- Opportunities table
CREATE TABLE opportunities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pair_address VARCHAR(42) NOT NULL,
    token_a_address VARCHAR(42) NOT NULL,
    token_b_address VARCHAR(42) NOT NULL,
    token_a_symbol VARCHAR(20) NOT NULL,
    token_b_symbol VARCHAR(20) NOT NULL,
    dex_a VARCHAR(50) NOT NULL,
    dex_b VARCHAR(50) NOT NULL,
    price_a DECIMAL(36, 18) NOT NULL,
    price_b DECIMAL(36, 18) NOT NULL,
    profit_potential DECIMAL(36, 18) NOT NULL,
    profit_percentage DECIMAL(10, 6) NOT NULL,
    gas_cost DECIMAL(36, 18) NOT NULL,
    net_profit DECIMAL(36, 18) NOT NULL,
    liquidity_a DECIMAL(36, 18) NOT NULL,
    liquidity_b DECIMAL(36, 18) NOT NULL,
    chain chain_type NOT NULL,
    status opportunity_status DEFAULT 'active',
    block_number BIGINT NOT NULL,
    transaction_hash VARCHAR(66),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    
    -- Constraints
    CONSTRAINT positive_profit CHECK (profit_potential >= 0),
    CONSTRAINT valid_addresses CHECK (
        pair_address ~ '^0x[a-fA-F0-9]{40}$' AND
        token_a_address ~ '^0x[a-fA-F0-9]{40}$' AND
        token_b_address ~ '^0x[a-fA-F0-9]{40}$'
    )
);

-- Executions table
CREATE TABLE executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    opportunity_id UUID NOT NULL REFERENCES opportunities(id) ON DELETE CASCADE,
    strategy_type strategy_type NOT NULL,
    amount_in DECIMAL(36, 18) NOT NULL,
    amount_out DECIMAL(36, 18),
    gas_used BIGINT,
    gas_price DECIMAL(36, 18),
    transaction_hash VARCHAR(66),
    block_number BIGINT,
    status execution_status DEFAULT 'pending',
    error_message TEXT,
    profit_realized DECIMAL(36, 18),
    slippage DECIMAL(10, 6),
    execution_time_ms INTEGER,
    chain chain_type NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    
    -- Constraints
    CONSTRAINT positive_amount CHECK (amount_in > 0),
    CONSTRAINT valid_tx_hash CHECK (transaction_hash IS NULL OR transaction_hash ~ '^0x[a-fA-F0-9]{64}$')
);

-- Strategies table
CREATE TABLE strategies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE,
    type strategy_type NOT NULL,
    description TEXT,
    parameters JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN DEFAULT true,
    min_profit_threshold DECIMAL(36, 18) NOT NULL DEFAULT 0,
    max_gas_price DECIMAL(36, 18),
    max_slippage DECIMAL(10, 6) DEFAULT 0.05,
    supported_chains chain_type[] NOT NULL,
    priority INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT positive_priority CHECK (priority >= 0),
    CONSTRAINT valid_slippage CHECK (max_slippage >= 0 AND max_slippage <= 1)
);

-- Analytics table
CREATE TABLE analytics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    date DATE NOT NULL,
    chain chain_type NOT NULL,
    total_opportunities INTEGER DEFAULT 0,
    successful_executions INTEGER DEFAULT 0,
    failed_executions INTEGER DEFAULT 0,
    total_volume DECIMAL(36, 18) DEFAULT 0,
    total_profit DECIMAL(36, 18) DEFAULT 0,
    total_gas_used BIGINT DEFAULT 0,
    average_profit_percentage DECIMAL(10, 6) DEFAULT 0,
    success_rate DECIMAL(5, 4) DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Unique constraint for daily analytics per chain
    UNIQUE(date, chain)
);

-- Tokens table
CREATE TABLE tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    address VARCHAR(42) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    name VARCHAR(100) NOT NULL,
    decimals INTEGER NOT NULL,
    chain chain_type NOT NULL,
    is_verified BOOLEAN DEFAULT false,
    price_usd DECIMAL(36, 18),
    market_cap DECIMAL(36, 18),
    volume_24h DECIMAL(36, 18),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Unique constraint per chain
    UNIQUE(address, chain),
    
    -- Constraints
    CONSTRAINT valid_token_address CHECK (address ~ '^0x[a-fA-F0-9]{40}$'),
    CONSTRAINT positive_decimals CHECK (decimals >= 0 AND decimals <= 18)
);

-- DEX pairs table
CREATE TABLE dex_pairs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    dex_name VARCHAR(50) NOT NULL,
    pair_address VARCHAR(42) NOT NULL,
    token_a_address VARCHAR(42) NOT NULL,
    token_b_address VARCHAR(42) NOT NULL,
    reserve_a DECIMAL(36, 18) NOT NULL DEFAULT 0,
    reserve_b DECIMAL(36, 18) NOT NULL DEFAULT 0,
    fee_percentage DECIMAL(10, 6) NOT NULL DEFAULT 0.003,
    chain chain_type NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Unique constraint
    UNIQUE(dex_name, pair_address, chain),
    
    -- Constraints
    CONSTRAINT valid_pair_address CHECK (pair_address ~ '^0x[a-fA-F0-9]{40}$'),
    CONSTRAINT positive_reserves CHECK (reserve_a >= 0 AND reserve_b >= 0),
    CONSTRAINT valid_fee CHECK (fee_percentage >= 0 AND fee_percentage <= 1)
);

-- =====================================================
-- INDEXES FOR PERFORMANCE
-- =====================================================

-- Opportunities indexes
CREATE INDEX idx_opportunities_status ON opportunities(status);
CREATE INDEX idx_opportunities_chain ON opportunities(chain);
CREATE INDEX idx_opportunities_created_at ON opportunities(created_at DESC);
CREATE INDEX idx_opportunities_profit ON opportunities(profit_potential DESC);
CREATE INDEX idx_opportunities_expires_at ON opportunities(expires_at);
CREATE INDEX idx_opportunities_tokens ON opportunities(token_a_address, token_b_address);
CREATE INDEX idx_opportunities_dexes ON opportunities(dex_a, dex_b);
CREATE INDEX idx_opportunities_composite ON opportunities(status, chain, created_at DESC);

-- Executions indexes
CREATE INDEX idx_executions_opportunity_id ON executions(opportunity_id);
CREATE INDEX idx_executions_status ON executions(status);
CREATE INDEX idx_executions_chain ON executions(chain);
CREATE INDEX idx_executions_created_at ON executions(created_at DESC);
CREATE INDEX idx_executions_tx_hash ON executions(transaction_hash);
CREATE INDEX idx_executions_profit ON executions(profit_realized DESC);

-- Strategies indexes
CREATE INDEX idx_strategies_type ON strategies(type);
CREATE INDEX idx_strategies_active ON strategies(is_active);
CREATE INDEX idx_strategies_priority ON strategies(priority DESC);

-- Analytics indexes
CREATE INDEX idx_analytics_date ON analytics(date DESC);
CREATE INDEX idx_analytics_chain ON analytics(chain);
CREATE UNIQUE INDEX idx_analytics_date_chain ON analytics(date, chain);

-- Tokens indexes
CREATE INDEX idx_tokens_address_chain ON tokens(address, chain);
CREATE INDEX idx_tokens_symbol ON tokens(symbol);
CREATE INDEX idx_tokens_verified ON tokens(is_verified);
CREATE INDEX idx_tokens_price ON tokens(price_usd DESC);

-- DEX pairs indexes
CREATE INDEX idx_dex_pairs_dex ON dex_pairs(dex_name);
CREATE INDEX idx_dex_pairs_tokens ON dex_pairs(token_a_address, token_b_address);
CREATE INDEX idx_dex_pairs_chain ON dex_pairs(chain);
CREATE INDEX idx_dex_pairs_active ON dex_pairs(is_active);

-- =====================================================
-- TRIGGERS FOR AUTOMATIC UPDATES
-- =====================================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply triggers to all tables
CREATE TRIGGER update_opportunities_updated_at BEFORE UPDATE ON opportunities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_executions_updated_at BEFORE UPDATE ON executions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_strategies_updated_at BEFORE UPDATE ON strategies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_analytics_updated_at BEFORE UPDATE ON analytics
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_tokens_updated_at BEFORE UPDATE ON tokens
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_dex_pairs_updated_at BEFORE UPDATE ON dex_pairs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- =====================================================
-- VIEWS FOR COMMON QUERIES
-- =====================================================

-- Active opportunities with profit ranking
CREATE VIEW v_active_opportunities AS
SELECT 
    o.*,
    ta.symbol as token_a_symbol_full,
    tb.symbol as token_b_symbol_full,
    RANK() OVER (PARTITION BY o.chain ORDER BY o.profit_potential DESC) as profit_rank
FROM opportunities o
LEFT JOIN tokens ta ON o.token_a_address = ta.address AND o.chain = ta.chain
LEFT JOIN tokens tb ON o.token_b_address = tb.address AND o.chain = tb.chain
WHERE o.status = 'active' AND o.expires_at > NOW();

-- Execution statistics
CREATE VIEW v_execution_stats AS
SELECT 
    DATE(created_at) as date,
    chain,
    strategy_type,
    COUNT(*) as total_executions,
    COUNT(CASE WHEN status = 'completed' THEN 1 END) as successful_executions,
    COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_executions,
    AVG(CASE WHEN status = 'completed' THEN profit_realized END) as avg_profit,
    SUM(CASE WHEN status = 'completed' THEN profit_realized ELSE 0 END) as total_profit,
    AVG(execution_time_ms) as avg_execution_time
FROM executions
GROUP BY DATE(created_at), chain, strategy_type;

-- Top performing pairs
CREATE VIEW v_top_pairs AS
SELECT 
    token_a_address,
    token_b_address,
    token_a_symbol,
    token_b_symbol,
    chain,
    COUNT(*) as opportunity_count,
    AVG(profit_percentage) as avg_profit_percentage,
    SUM(CASE WHEN status = 'executed' THEN 1 ELSE 0 END) as executed_count
FROM opportunities
WHERE created_at >= NOW() - INTERVAL '7 days'
GROUP BY token_a_address, token_b_address, token_a_symbol, token_b_symbol, chain
ORDER BY opportunity_count DESC, avg_profit_percentage DESC;

-- =====================================================
-- STORED PROCEDURES
-- =====================================================

-- Clean up expired opportunities
CREATE OR REPLACE FUNCTION cleanup_expired_opportunities()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM opportunities 
    WHERE status = 'active' AND expires_at < NOW() - INTERVAL '1 hour';
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    INSERT INTO analytics (date, chain, total_opportunities) 
    VALUES (CURRENT_DATE, 'ethereum', -deleted_count)
    ON CONFLICT (date, chain) 
    DO UPDATE SET total_opportunities = analytics.total_opportunities - deleted_count;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Update daily analytics
CREATE OR REPLACE FUNCTION update_daily_analytics()
RETURNS VOID AS $$
BEGIN
    INSERT INTO analytics (
        date, 
        chain, 
        total_opportunities, 
        successful_executions, 
        failed_executions,
        total_volume,
        total_profit,
        total_gas_used,
        average_profit_percentage,
        success_rate
    )
    SELECT 
        CURRENT_DATE,
        o.chain,
        COUNT(o.id),
        COUNT(CASE WHEN e.status = 'completed' THEN 1 END),
        COUNT(CASE WHEN e.status = 'failed' THEN 1 END),
        COALESCE(SUM(CASE WHEN e.status = 'completed' THEN e.amount_in END), 0),
        COALESCE(SUM(CASE WHEN e.status = 'completed' THEN e.profit_realized END), 0),
        COALESCE(SUM(CASE WHEN e.status = 'completed' THEN e.gas_used END), 0),
        AVG(o.profit_percentage),
        CASE 
            WHEN COUNT(e.id) > 0 THEN 
                COUNT(CASE WHEN e.status = 'completed' THEN 1 END)::DECIMAL / COUNT(e.id)
            ELSE 0 
        END
    FROM opportunities o
    LEFT JOIN executions e ON o.id = e.opportunity_id
    WHERE DATE(o.created_at) = CURRENT_DATE
    GROUP BY o.chain
    ON CONFLICT (date, chain) 
    DO UPDATE SET
        total_opportunities = EXCLUDED.total_opportunities,
        successful_executions = EXCLUDED.successful_executions,
        failed_executions = EXCLUDED.failed_executions,
        total_volume = EXCLUDED.total_volume,
        total_profit = EXCLUDED.total_profit,
        total_gas_used = EXCLUDED.total_gas_used,
        average_profit_percentage = EXCLUDED.average_profit_percentage,
        success_rate = EXCLUDED.success_rate,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- =====================================================
-- D1 MIGRATIONS (Edge/Cloudflare)
-- Ubicación: d1/migrations/0001_initial_schema.sql
-- =====================================================

-- D1 Schema (SQLite-compatible)
CREATE TABLE IF NOT EXISTS opportunities_cache (
    id TEXT PRIMARY KEY,
    pair_address TEXT NOT NULL,
    token_a_symbol TEXT NOT NULL,
    token_b_symbol TEXT NOT NULL,
    dex_a TEXT NOT NULL,
    dex_b TEXT NOT NULL,
    profit_potential REAL NOT NULL,
    profit_percentage REAL NOT NULL,
    chain TEXT NOT NULL,
    status TEXT DEFAULT 'active',
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS executions_cache (
    id TEXT PRIMARY KEY,
    opportunity_id TEXT NOT NULL,
    strategy_type TEXT NOT NULL,
    amount_in REAL NOT NULL,
    profit_realized REAL,
    status TEXT DEFAULT 'pending',
    chain TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (opportunity_id) REFERENCES opportunities_cache(id)
);

CREATE TABLE IF NOT EXISTS analytics_cache (
    id TEXT PRIMARY KEY,
    date TEXT NOT NULL,
    chain TEXT NOT NULL,
    total_opportunities INTEGER DEFAULT 0,
    successful_executions INTEGER DEFAULT 0,
    total_profit REAL DEFAULT 0,
    success_rate REAL DEFAULT 0,
    created_at INTEGER NOT NULL,
    UNIQUE(date, chain)
);

CREATE TABLE IF NOT EXISTS rate_limits (
    id TEXT PRIMARY KEY,
    ip_address TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    request_count INTEGER DEFAULT 0,
    window_start INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(ip_address, endpoint)
);

-- D1 Indexes
CREATE INDEX IF NOT EXISTS idx_opportunities_cache_status ON opportunities_cache(status);
CREATE INDEX IF NOT EXISTS idx_opportunities_cache_chain ON opportunities_cache(chain);
CREATE INDEX IF NOT EXISTS idx_opportunities_cache_created_at ON opportunities_cache(created_at);
CREATE INDEX IF NOT EXISTS idx_opportunities_cache_profit ON opportunities_cache(profit_potential);

CREATE INDEX IF NOT EXISTS idx_executions_cache_opportunity_id ON executions_cache(opportunity_id);
CREATE INDEX IF NOT EXISTS idx_executions_cache_status ON executions_cache(status);
CREATE INDEX IF NOT EXISTS idx_executions_cache_chain ON executions_cache(chain);

CREATE INDEX IF NOT EXISTS idx_analytics_cache_date ON analytics_cache(date);
CREATE INDEX IF NOT EXISTS idx_analytics_cache_chain ON analytics_cache(chain);

CREATE INDEX IF NOT EXISTS idx_rate_limits_ip ON rate_limits(ip_address);
CREATE INDEX IF NOT EXISTS idx_rate_limits_window ON rate_limits(window_start);

-- =====================================================
-- MIGRATION SCRIPTS
-- =====================================================

-- PostgreSQL Migration Script
-- Ubicación: scripts/migrate-postgres.sh
/*
#!/bin/bash
set -e

echo "🗄️ Running PostgreSQL migrations..."

# Check if database exists
if ! psql -lqt | cut -d \| -f 1 | grep -qw arbitragex_prod; then
    echo "Creating database arbitragex_prod..."
    createdb arbitragex_prod
fi

# Run migrations
psql -d arbitragex_prod -f migrations/001_initial_schema.sql

# Verify migration
psql -d arbitragex_prod -c "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public';"

echo "✅ PostgreSQL migrations completed successfully!"
*/

-- D1 Migration Script
-- Ubicación: scripts/migrate-d1.sh
/*
#!/bin/bash
set -e

echo "🗄️ Running D1 migrations..."

# Apply migrations to staging
wrangler d1 migrations apply arbitragex-db-staging --env staging

# Apply migrations to production
wrangler d1 migrations apply arbitragex-db-prod --env production

# Verify migrations
wrangler d1 execute arbitragex-db-prod --command="SELECT name FROM sqlite_master WHERE type='table';" --env production

echo "✅ D1 migrations completed successfully!"
*/

-- =====================================================
-- SEED DATA FOR TESTING
-- =====================================================

-- Insert default strategies
INSERT INTO strategies (name, type, description, parameters, supported_chains) VALUES
('Basic Arbitrage', 'arbitrage', 'Simple price difference arbitrage between DEXes', '{"min_profit_usd": 10}', ARRAY['ethereum', 'polygon']),
('Flashloan Arbitrage', 'flashloan', 'Arbitrage using flashloans for larger capital', '{"min_profit_usd": 50, "max_loan_amount": 1000000}', ARRAY['ethereum', 'arbitrum']),
('MEV Protection', 'mev', 'MEV-protected arbitrage execution', '{"use_private_mempool": true}', ARRAY['ethereum']),
('Cross-chain Arbitrage', 'cross_chain', 'Arbitrage across different chains', '{"bridge_fee_threshold": 0.1}', ARRAY['ethereum', 'polygon', 'arbitrum']);

-- Insert common tokens
INSERT INTO tokens (address, symbol, name, decimals, chain, is_verified) VALUES
('0xA0b86a33E6441b8435b662b0C0C39C1A5b0c5c3c', 'WETH', 'Wrapped Ether', 18, 'ethereum', true),
('0x6B175474E89094C44Da98b954EedeAC495271d0F', 'DAI', 'Dai Stablecoin', 18, 'ethereum', true),
('0xA0b86a33E6441b8435b662b0C0C39C1A5b0c5c3c', 'USDC', 'USD Coin', 6, 'ethereum', true),
('0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599', 'WBTC', 'Wrapped Bitcoin', 8, 'ethereum', true);

COMMIT;
