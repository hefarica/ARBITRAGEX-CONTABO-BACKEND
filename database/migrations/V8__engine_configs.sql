CREATE TABLE IF NOT EXISTS engine_configs (
  id SERIAL PRIMARY KEY,
  version TEXT NOT NULL,
  config JSONB NOT NULL,
  updated_by TEXT,
  updated_at TIMESTAMP DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_engine_configs_version ON engine_configs(version);
