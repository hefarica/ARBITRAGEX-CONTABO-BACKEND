-- Extensiones requeridas
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm"; -- Para búsquedas de texto más eficientes

-- Tabla de blockchains soportadas
CREATE TABLE IF NOT EXISTS blockchains (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    chain_id INTEGER UNIQUE NOT NULL,
    rpc_url TEXT NOT NULL,
    backup_rpc_urls TEXT[] NOT NULL DEFAULT '{}',
    ws_url TEXT,
    explorer_url TEXT NOT NULL,
    native_token VARCHAR(10) NOT NULL,
    tvl DECIMAL(24, 2),
    block_time DECIMAL(10, 2), -- tiempo promedio en segundos
    flash_loan_support BOOLEAN NOT NULL DEFAULT false,
    priority INTEGER NOT NULL DEFAULT 3, -- 1 (alta), 2 (media), 3 (baja)
    update_interval INTEGER NOT NULL DEFAULT 6, -- segundos entre actualizaciones
    active BOOLEAN NOT NULL DEFAULT true,
    verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Tabla de métricas de RPC
CREATE TABLE IF NOT EXISTS rpc_metrics (
    id SERIAL PRIMARY KEY,
    blockchain_id INTEGER REFERENCES blockchains(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    latency INTEGER, -- ms
    success BOOLEAN NOT NULL,
    error_message TEXT,
    response_time DECIMAL(10, 3), -- segundos con 3 decimales
    current_block BIGINT,
    uptime_score DECIMAL(5, 2), -- 0-100
    measured_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_rpc_metrics_blockchain_id ON rpc_metrics(blockchain_id);
CREATE INDEX IF NOT EXISTS idx_rpc_metrics_measured_at ON rpc_metrics(measured_at);

-- Tabla de oportunidades de arbitraje
CREATE TABLE IF NOT EXISTS arbitrage_opportunities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_blockchain_id INTEGER REFERENCES blockchains(id),
    target_blockchain_id INTEGER REFERENCES blockchains(id),
    source_token VARCHAR(42) NOT NULL,
    target_token VARCHAR(42) NOT NULL,
    source_amount VARCHAR(78) NOT NULL, -- valores grandes con precisión
    target_amount VARCHAR(78) NOT NULL,
    profit_usd DECIMAL(20, 2) NOT NULL,
    profit_percentage DECIMAL(10, 2) NOT NULL,
    gas_cost_usd DECIMAL(10, 2) NOT NULL,
    route JSONB NOT NULL, -- ruta completa incluyendo DEXs
    execution_data JSONB, -- datos para ejecutar la transacción
    status VARCHAR(20) NOT NULL DEFAULT 'detected', -- detected, simulated, executing, completed, failed
    valid_until TIMESTAMP WITH TIME ZONE NOT NULL,
    detected_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_opportunities_status ON arbitrage_opportunities(status);
CREATE INDEX IF NOT EXISTS idx_opportunities_profit ON arbitrage_opportunities(profit_usd DESC);
CREATE INDEX IF NOT EXISTS idx_opportunities_detected ON arbitrage_opportunities(detected_at DESC);

-- Tabla de ejecuciones
CREATE TABLE IF NOT EXISTS executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    opportunity_id UUID REFERENCES arbitrage_opportunities(id),
    transaction_hash VARCHAR(66),
    status VARCHAR(20) NOT NULL, -- pending, sent, confirmed, failed
    gas_used BIGINT,
    actual_profit_usd DECIMAL(20, 2),
    block_number BIGINT,
    error_message TEXT,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP WITH TIME ZONE
);
CREATE INDEX IF NOT EXISTS idx_executions_opportunity ON executions(opportunity_id);
CREATE INDEX IF NOT EXISTS idx_executions_status ON executions(status);

-- Función para actualizar el timestamp updated_at
CREATE OR REPLACE FUNCTION update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger para blockchains
CREATE TRIGGER update_blockchains_timestamp
BEFORE UPDATE ON blockchains
FOR EACH ROW
EXECUTE FUNCTION update_timestamp();

-- Insertar algunas blockchains iniciales de la Top 10 Elite
INSERT INTO blockchains (name, chain_id, rpc_url, backup_rpc_urls, ws_url, explorer_url, native_token, tvl, block_time, flash_loan_support, priority, update_interval, verified) VALUES
('Ethereum', 1, 'https://eth.llamarpc.com', ARRAY['https://cloudflare-eth.com', 'https://1rpc.io/eth'], 'wss://eth.llamarpc.com', 'https://etherscan.io', 'ETH', 50307845823.45, 12.00, true, 1, 2, true),
('Arbitrum', 42161, 'https://arb1.arbitrum.io/rpc', ARRAY['https://arbitrum-one.publicnode.com'], 'wss://arb1.arbitrum.io/ws', 'https://arbiscan.io', 'ETH', 2165478963.67, 0.30, true, 1, 2, true),
('Base', 8453, 'https://mainnet.base.org', ARRAY['https://base.llamarpc.com'], 'wss://base.llamarpc.com', 'https://basescan.org', 'ETH', 1356897412.89, 2.00, true, 1, 2, true),
('Optimism', 10, 'https://mainnet.optimism.io', ARRAY['https://optimism.publicnode.com'], 'wss://optimism.publicnode.com', 'https://optimistic.etherscan.io', 'ETH', 987456321.43, 2.00, true, 1, 2, true),
('Polygon', 137, 'https://polygon-rpc.com', ARRAY['https://polygon.llamarpc.com'], 'wss://polygon.llamarpc.com', 'https://polygonscan.com', 'MATIC', 1452369874.25, 2.20, true, 1, 2, true),
('BNB Chain', 56, 'https://bsc-dataseed.binance.org', ARRAY['https://bsc-dataseed1.defibit.io'], 'wss://bsc-ws-node.nariox.org', 'https://bscscan.com', 'BNB', 3654789123.88, 3.00, true, 1, 2, true),
('Avalanche', 43114, 'https://api.avax.network/ext/bc/C/rpc', ARRAY['https://avalanche.public-rpc.com'], 'wss://api.avax.network/ext/bc/C/ws', 'https://snowtrace.io', 'AVAX', 784512369.56, 2.00, true, 1, 2, true),
('Fantom', 250, 'https://rpcapi.fantom.network', ARRAY['https://fantom.publicnode.com'], 'wss://wsapi.fantom.network', 'https://ftmscan.com', 'FTM', 125896347.42, 1.00, true, 1, 2, true),
('zkSync Era', 324, 'https://mainnet.era.zksync.io', ARRAY['https://zksync.meowrpc.com'], NULL, 'https://explorer.zksync.io', 'ETH', 789456123.01, 1.00, true, 2, 4, true),
('Linea', 59144, 'https://rpc.linea.build', ARRAY['https://linea.publicnode.com'], 'wss://linea.publicnode.com', 'https://lineascan.build', 'ETH', 97561234.78, 3.00, true, 2, 4, true);
