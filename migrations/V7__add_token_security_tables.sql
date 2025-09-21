-- ArbitrageX Supreme V3.3 (RLI) - Migration V7
-- Añade tablas de seguridad de tokens para el sistema Anti-Rug

-- Tabla de información de seguridad de tokens
CREATE TABLE IF NOT EXISTS tokens_security (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    token_address VARCHAR(255) NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    creation_date TIMESTAMP WITH TIME ZONE,
    holder_count BIGINT,
    verified_source BOOLEAN DEFAULT FALSE,
    is_mintable BOOLEAN DEFAULT FALSE,
    has_proxy BOOLEAN DEFAULT FALSE,
    owner_address VARCHAR(255),
    owner_percent DECIMAL(5, 2),
    verified_on_exchanges TEXT[], -- Lista de exchanges donde está listado
    security_audit_auditor VARCHAR(255),
    security_audit_date TIMESTAMP WITH TIME ZONE,
    security_audit_report_url TEXT,
    security_audit_issues_found INTEGER,
    security_audit_issues_fixed INTEGER,
    security_audit_score INTEGER, -- 0-100
    security_score INTEGER NOT NULL, -- 0-100
    warning_flags TEXT[],
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(chain_id, token_address)
);

-- Tabla de holders principales de tokens
CREATE TABLE IF NOT EXISTS token_holders (
    id SERIAL PRIMARY KEY,
    token_security_id INTEGER REFERENCES tokens_security(id) ON DELETE CASCADE,
    holder_address VARCHAR(255) NOT NULL,
    percent DECIMAL(5, 2) NOT NULL,
    is_contract BOOLEAN DEFAULT FALSE,
    is_locked BOOLEAN DEFAULT FALSE,
    lock_end_date TIMESTAMP WITH TIME ZONE,
    is_exchange BOOLEAN DEFAULT FALSE,
    UNIQUE(token_security_id, holder_address)
);

-- Tabla de distribución de liquidez de tokens
CREATE TABLE IF NOT EXISTS token_liquidity (
    id SERIAL PRIMARY KEY,
    token_security_id INTEGER REFERENCES tokens_security(id) ON DELETE CASCADE,
    dex_name VARCHAR(100) NOT NULL,
    chain_id INTEGER NOT NULL,
    pair_address VARCHAR(255) NOT NULL,
    pair_with VARCHAR(50) NOT NULL, -- Symbol del otro token
    liquidity_usd DECIMAL(20, 2) NOT NULL,
    is_locked BOOLEAN DEFAULT FALSE,
    lock_end_date TIMESTAMP WITH TIME ZONE,
    percent_locked DECIMAL(5, 2),
    UNIQUE(token_security_id, dex_name, chain_id, pair_address)
);

-- Tabla de blacklist de tokens
CREATE TABLE IF NOT EXISTS tokens_blacklist (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    token_address VARCHAR(255) NOT NULL,
    reason TEXT NOT NULL,
    added_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    added_by VARCHAR(100),
    UNIQUE(chain_id, token_address)
);

-- Tabla de whitelist de tokens (verificados manualmente)
CREATE TABLE IF NOT EXISTS tokens_whitelist (
    id SERIAL PRIMARY KEY,
    chain_id INTEGER NOT NULL,
    token_address VARCHAR(255) NOT NULL,
    verification_source VARCHAR(255) NOT NULL,
    verified_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    verified_by VARCHAR(100),
    UNIQUE(chain_id, token_address)
);

-- Índices para optimizar consultas
CREATE INDEX IF NOT EXISTS idx_tokens_security_chain_id ON tokens_security(chain_id);
CREATE INDEX IF NOT EXISTS idx_tokens_security_score ON tokens_security(security_score);
CREATE INDEX IF NOT EXISTS idx_tokens_security_updated ON tokens_security(last_updated);
CREATE INDEX IF NOT EXISTS idx_token_holders_percent ON token_holders(percent);
CREATE INDEX IF NOT EXISTS idx_token_liquidity_usd ON token_liquidity(liquidity_usd);
CREATE INDEX IF NOT EXISTS idx_tokens_blacklist_chain ON tokens_blacklist(chain_id);
CREATE INDEX IF NOT EXISTS idx_tokens_whitelist_chain ON tokens_whitelist(chain_id);
