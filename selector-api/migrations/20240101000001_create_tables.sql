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
CREATE TABLE IF NOT EXISTS opportunities (
    id UUID PRIMARY KEY,
    chain VARCHAR(50) NOT NULL,
    strategy_type VARCHAR(50) NOT NULL,
    tokens TEXT[] NOT NULL,
    pools TEXT[] NOT NULL,
    expected_profit BIGINT NOT NULL,
    gas_cost BIGINT NOT NULL,
    confidence DOUBLE PRECISION NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    deadline BIGINT NOT NULL,
    status opportunity_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create executions table
CREATE TABLE IF NOT EXISTS executions (
    id UUID PRIMARY KEY,
    opportunity_id UUID NOT NULL REFERENCES opportunities(id),
    tx_hash VARCHAR(66),
    status execution_status NOT NULL DEFAULT 'pending',
    actual_profit BIGINT,
    gas_used BIGINT,
    error_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
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

-- Create indexes
CREATE INDEX idx_opportunities_status ON opportunities(status);
CREATE INDEX idx_opportunities_deadline ON opportunities(deadline);
CREATE INDEX idx_opportunities_profit ON opportunities(expected_profit DESC);
CREATE INDEX idx_executions_opportunity ON executions(opportunity_id);
CREATE INDEX idx_executions_status ON executions(status);
CREATE INDEX idx_pools_active ON pools(active);

-- Create update trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_opportunities_updated_at BEFORE UPDATE
    ON opportunities FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_pools_updated_at BEFORE UPDATE
    ON pools FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();



