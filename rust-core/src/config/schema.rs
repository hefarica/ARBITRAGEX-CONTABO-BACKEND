use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineConfig {
    pub enabled: bool,
    pub version: String,
    pub dry_run: bool,
    pub strategies: Strategies,
    pub funding: Funding,
    pub risk: Risk,
    pub liquidity: Liquidity,
    pub discovery: Discovery,
    pub execution: Execution,
    pub providers: Providers,
    pub chains: Chains,
    pub alerts: Alerts,
    pub assets_policy: AssetsPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Strategies {
    pub enabled: StrategyFlags,
    pub priority: Vec<String>,
    pub roi_min_bps: u32,
    pub ev_min_usd: f64,
    pub max_hops: MaxHops,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyFlags {
    pub dex_dex: bool,
    pub triangular_inter_dex: bool,
    pub triangular_intra_dex: bool,
    pub stable_arb: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaxHops {
    pub dex_dex: u8,
    pub triangular_inter_dex: u8,
    pub triangular_intra_dex: u8,
    pub stable_arb: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Funding {
    pub mode: String,
    pub lenders: HashMap<String, Vec<Lender>>,
    pub assets: HashMap<String, Vec<String>>,
    pub amount_bands: HashMap<String, Vec<AmountBand>>,
    pub fee_bps_ceiling: u32,
    pub atomic_repay_required: bool,
    pub fallback_on_insufficient_liquidity: bool,
    pub retries: Retries,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lender {
    pub name: String,
    pub protocol: String,
    pub contract: String,
    pub priority: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmountBand {
    pub asset: String,
    pub min_usd: f64,
    pub max_usd: f64,
    pub step_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Retries {
    pub max_attempts: u8,
    pub backoff_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Risk {
    pub risk_max: u8,
    pub tax_max_bps: u32,
    pub honeypot_check: bool,
    pub deny_fee_on_transfer: bool,
    pub allowlist: AllowDenyList,
    pub denylist: AllowDenyList,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllowDenyList {
    pub tokens: Vec<String>,
    pub pairs: Vec<String>,
    pub dexes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Liquidity {
    pub size_usd_candidates: Vec<f64>,
    pub max_slippage_bps: u32,
    pub min_depth_usd: f64,
    pub twap_window_sec: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discovery {
    pub top_pairs_per_chain: u16,
    pub scan_interval_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Execution {
    pub route_mode: String,
    pub private_relays: Vec<String>,
    pub prob_exec_priv_min: f64,
    pub gas: GasPolicy,
    pub size_dynamic: bool,
    pub kill_switch: bool,
    pub path_templates: PathTemplates,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasPolicy {
    pub policy: String,
    pub percentile: u8,
    pub max_fee_cap_gwei: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathTemplates {
    pub dex_dex: String,
    pub triangular_inter_dex: String,
    pub triangular_intra_dex: String,
    pub stable_arb: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Providers {
    pub base: ChainProvider,
    pub arbitrum: ChainProvider,
    pub polygon: ChainProvider,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainProvider {
    pub ws: Vec<WsEndpoint>,
    pub http: Vec<HttpEndpoint>,
    pub multicall: MulticallConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsEndpoint {
    pub url: String,
    pub weight: u32,
    pub timeout_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpEndpoint {
    pub url: String,
    pub weight: u32,
    pub timeout_ms: u32,
    pub max_rps: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MulticallConfig {
    pub batch_size: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chains {
    pub base: ChainOverride,
    pub arbitrum: ChainOverride,
    pub polygon: ChainOverride,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainOverride {
    pub enabled: bool,
    pub overrides: Option<ChainOverrides>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainOverrides {
    pub liquidity: Option<Liquidity>,
    pub funding: Option<Funding>,
    pub risk: Option<Risk>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alerts {
    pub webhook_url: Option<String>,
    pub thresholds: Thresholds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thresholds {
    pub ev_min_notify: f64,
    pub detect_to_publish_p95_ms: u32,
    pub private_inclusion_success_min: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetsPolicy {
    pub min_age_days: u32,
    pub require_oracle: bool,
    pub min_liquidity_usd: f64,
    pub min_daily_volume_usd: f64,
    pub deny_fee_on_transfer: bool,
    pub deny_blacklist: bool,
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
    pub preferred_stablecoins: Vec<String>,
    pub preferred_bluechips: Vec<String>,
}
