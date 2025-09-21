-- ArbitrageX Supreme V3.3 (RLI) - Migration V6
-- Añade tablas para análisis de liquidez

-- Tabla principal de análisis de profundidad de liquidez
CREATE TABLE IF NOT EXISTS liquidity_depth_analysis (
    id SERIAL PRIMARY KEY,
    pair_id VARCHAR(255) NOT NULL,
    chain_id INTEGER NOT NULL,
    dex_name VARCHAR(100) NOT NULL,
    token0_symbol VARCHAR(50) NOT NULL,
    token1_symbol VARCHAR(50) NOT NULL,
    avg_slippage_per_1k_usd DECIMAL(10, 6),
    max_single_trade_usd DECIMAL(20, 2),
    liquidity_distribution_score INTEGER, -- 0-100
    overall_health_score INTEGER, -- 0-100
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain_id, pair_id)
);

-- Tabla de métricas de impacto de precio
CREATE TABLE IF NOT EXISTS price_impact_metrics (
    id SERIAL PRIMARY KEY,
    liquidity_analysis_id INTEGER REFERENCES liquidity_depth_analysis(id) ON DELETE CASCADE,
    amount_usd DECIMAL(20, 2) NOT NULL,
    impact_percent DECIMAL(10, 6) NOT NULL,
    direction VARCHAR(4) NOT NULL CHECK (direction IN ('buy', 'sell')),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Tabla de concentración de liquidez (principalmente para pools V3)
CREATE TABLE IF NOT EXISTS liquidity_concentration (
    id SERIAL PRIMARY KEY,
    pair_id VARCHAR(255) NOT NULL,
    chain_id INTEGER NOT NULL,
    concentration_score INTEGER NOT NULL, -- 0-100
    imbalance_ratio DECIMAL(10, 6),
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain_id, pair_id)
);

-- Tabla de rangos de precio para liquidez concentrada
CREATE TABLE IF NOT EXISTS price_ranges (
    id SERIAL PRIMARY KEY,
    concentration_id INTEGER REFERENCES liquidity_concentration(id) ON DELETE CASCADE,
    lower_price DECIMAL(30, 18) NOT NULL,
    upper_price DECIMAL(30, 18) NOT NULL,
    liquidity_percent DECIMAL(5, 2) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE
);

-- Tabla de histórico de liquidez
CREATE TABLE IF NOT EXISTS liquidity_history (
    id SERIAL PRIMARY KEY,
    pair_id VARCHAR(255) NOT NULL,
    chain_id INTEGER NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    liquidity_usd DECIMAL(20, 2) NOT NULL,
    volatility_24h DECIMAL(10, 6),
    trend VARCHAR(10) CHECK (trend IN ('increasing', 'stable', 'decreasing', 'volatile'))
);

-- Índices para optimizar consultas
CREATE INDEX IF NOT EXISTS idx_liquidity_depth_chain_dex ON liquidity_depth_analysis(chain_id, dex_name);
CREATE INDEX IF NOT EXISTS idx_liquidity_depth_score ON liquidity_depth_analysis(overall_health_score);
CREATE INDEX IF NOT EXISTS idx_liquidity_depth_updated ON liquidity_depth_analysis(last_updated);
CREATE INDEX IF NOT EXISTS idx_price_impact_amount ON price_impact_metrics(amount_usd);
CREATE INDEX IF NOT EXISTS idx_liquidity_concentration_score ON liquidity_concentration(concentration_score);
CREATE INDEX IF NOT EXISTS idx_price_ranges_active ON price_ranges(is_active);
CREATE INDEX IF NOT EXISTS idx_liquidity_history_timestamp ON liquidity_history(timestamp);
CREATE INDEX IF NOT EXISTS idx_liquidity_history_pair_chain ON liquidity_history(pair_id, chain_id);
