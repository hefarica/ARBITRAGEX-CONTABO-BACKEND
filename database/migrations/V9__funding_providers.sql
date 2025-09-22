CREATE TABLE IF NOT EXISTS funding_providers (
  id SERIAL PRIMARY KEY,
  chain_id TEXT NOT NULL,
  name TEXT NOT NULL,
  protocol TEXT NOT NULL,
  contract TEXT NOT NULL,
  assets JSONB NOT NULL DEFAULT '[]',
  fee_bps INTEGER DEFAULT 0,
  meta JSONB DEFAULT '{}',
  updated_at TIMESTAMP DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_funding_chain ON funding_providers(chain_id);
CREATE INDEX IF NOT EXISTS idx_funding_assets ON funding_providers USING GIN(assets);
