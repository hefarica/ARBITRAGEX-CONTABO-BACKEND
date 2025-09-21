-- ArbitrageX Supreme V3.3 (RLI) - Migration V5
-- Añade tablas para oráculos de precios

-- Tabla de configuración de oráculos
CREATE TABLE IF NOT EXISTS oracle_configs (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    oracle_type VARCHAR(50) NOT NULL CHECK (
        oracle_type IN ('chainlink', 'uniswap_v3_twap', 'sushiswap_twap', 'band_protocol', 'makerdao', 'api', 'composite')
    ),
    address VARCHAR(255),
    api_url TEXT,
    base_token VARCHAR(50),
    quote_token VARCHAR(50),
    decimals INTEGER NOT NULL,
    heartbeat_sec INTEGER NOT NULL,
    priority INTEGER NOT NULL DEFAULT 100,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain_id, oracle_type, base_token, quote_token)
);

-- Tabla de precios de oráculos individuales
CREATE TABLE IF NOT EXISTS oracle_prices (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    oracle_type VARCHAR(50) NOT NULL,
    base_token VARCHAR(50) NOT NULL,
    quote_token VARCHAR(50) NOT NULL,
    price DECIMAL(30, 18) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    confidence DECIMAL(3, 2) NOT NULL, -- 0-1
    deviation_24h DECIMAL(10, 6),
    raw_data JSONB,
    UNIQUE(chain_id, oracle_type, base_token, quote_token, timestamp)
);

-- Tabla de precios verificados (consenso de múltiples oráculos)
CREATE TABLE IF NOT EXISTS verified_prices (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    base_token VARCHAR(50) NOT NULL,
    quote_token VARCHAR(50) NOT NULL,
    price DECIMAL(30, 18) NOT NULL,
    confidence DECIMAL(3, 2) NOT NULL, -- 0-1
    source_count INTEGER NOT NULL,
    is_fresh BOOLEAN NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    max_deviation_percent DECIMAL(10, 6),
    UNIQUE(chain_id, base_token, quote_token, timestamp)
);

-- Tabla de verificaciones de precio para oportunidades
CREATE TABLE IF NOT EXISTS price_verifications (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    opportunity_id VARCHAR(255),
    token_a VARCHAR(50) NOT NULL,
    token_b VARCHAR(50) NOT NULL,
    dex_price_ratio DECIMAL(30, 18) NOT NULL,
    oracle_price DECIMAL(30, 18) NOT NULL,
    confidence_score INTEGER NOT NULL, -- 0-100
    deviation_percent DECIMAL(10, 6) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Tabla de histórico de precios por intervalo (para análisis)
CREATE TABLE IF NOT EXISTS price_history_intervals (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    base_token VARCHAR(50) NOT NULL,
    quote_token VARCHAR(50) NOT NULL,
    interval_type VARCHAR(10) NOT NULL CHECK (interval_type IN ('5m', '15m', '1h', '4h', '1d')),
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    open_price DECIMAL(30, 18) NOT NULL,
    high_price DECIMAL(30, 18) NOT NULL,
    low_price DECIMAL(30, 18) NOT NULL,
    close_price DECIMAL(30, 18) NOT NULL,
    volume DECIMAL(30, 18),
    UNIQUE(chain_id, base_token, quote_token, interval_type, timestamp)
);

-- Índices para optimizar consultas
CREATE INDEX IF NOT EXISTS idx_oracle_configs_chain_tokens ON oracle_configs(chain_id, base_token, quote_token);
CREATE INDEX IF NOT EXISTS idx_oracle_configs_priority ON oracle_configs(priority);
CREATE INDEX IF NOT EXISTS idx_oracle_prices_timestamp ON oracle_prices(timestamp);
CREATE INDEX IF NOT EXISTS idx_oracle_prices_chain_tokens ON oracle_prices(chain_id, base_token, quote_token);
CREATE INDEX IF NOT EXISTS idx_verified_prices_tokens ON verified_prices(base_token, quote_token);
CREATE INDEX IF NOT EXISTS idx_verified_prices_timestamp ON verified_prices(timestamp);
CREATE INDEX IF NOT EXISTS idx_price_verifications_opportunity ON price_verifications(opportunity_id);
CREATE INDEX IF NOT EXISTS idx_price_history_timestamp ON price_history_intervals(timestamp);
CREATE INDEX IF NOT EXISTS idx_price_history_tokens ON price_history_intervals(base_token, quote_token);
