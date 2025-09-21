-- ArbitrageX Supreme V3.3 (RLI) - Migration V4
-- Añade columnas para evaluación de riesgos en oportunidades y mejora el esquema

-- Añadir columnas de evaluación de riesgo a oportunidades
ALTER TABLE IF EXISTS arbitrage_opportunities 
    ADD COLUMN IF NOT EXISTS security_score INTEGER,
    ADD COLUMN IF NOT EXISTS liquidity_score INTEGER,
    ADD COLUMN IF NOT EXISTS price_confidence_score INTEGER,
    ADD COLUMN IF NOT EXISTS combined_risk_score INTEGER,
    ADD COLUMN IF NOT EXISTS risk_assessment TEXT;
    
-- Añadir columnas para controles de verificación
ALTER TABLE IF EXISTS arbitrage_opportunities
    ADD COLUMN IF NOT EXISTS verified_by_oracle BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS verified_by_security BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS verified_by_liquidity BOOLEAN DEFAULT FALSE;
    
-- Añadir columna para optimización de tamaño de trade
ALTER TABLE IF EXISTS arbitrage_opportunities
    ADD COLUMN IF NOT EXISTS optimal_trade_size_usd DECIMAL(20, 2);
    
-- Crear tabla para resultados de simulación
CREATE TABLE IF NOT EXISTS opportunity_simulations (
    id SERIAL PRIMARY KEY,
    opportunity_id VARCHAR(255) NOT NULL REFERENCES arbitrage_opportunities(id) ON DELETE CASCADE,
    simulated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    trade_size_usd DECIMAL(20, 2) NOT NULL,
    expected_profit_usd DECIMAL(20, 6) NOT NULL,
    actual_profit_usd DECIMAL(20, 6),
    slippage_percent DECIMAL(10, 6),
    gas_used BIGINT,
    simulation_time_ms INTEGER,
    successful BOOLEAN NOT NULL,
    error_message TEXT,
    transaction_trace JSONB
);

-- Índices para optimizar consultas
CREATE INDEX IF NOT EXISTS idx_opportunities_security_score ON arbitrage_opportunities(security_score);
CREATE INDEX IF NOT EXISTS idx_opportunities_liquidity_score ON arbitrage_opportunities(liquidity_score);
CREATE INDEX IF NOT EXISTS idx_opportunities_price_confidence ON arbitrage_opportunities(price_confidence_score);
CREATE INDEX IF NOT EXISTS idx_opportunities_combined_risk ON arbitrage_opportunities(combined_risk_score);
CREATE INDEX IF NOT EXISTS idx_opportunity_simulations_opportunity ON opportunity_simulations(opportunity_id);
CREATE INDEX IF NOT EXISTS idx_opportunity_simulations_success ON opportunity_simulations(successful);
