//! # Arbitrage Types and Data Structures
//! 
//! Core data structures and types used throughout the arbitrage system.

use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Arbitrage opportunity structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Unique opportunity identifier
    pub id: String,
    /// Strategy type being used
    pub strategy_type: StrategyType,
    /// Input token address
    pub token_in: Address,
    /// Output token address  
    pub token_out: Address,
    /// Input amount
    pub amount_in: U256,
    /// Expected output amount
    pub amount_out: U256,
    /// Estimated profit in ETH
    pub estimated_profit: f64,
    /// Gas cost estimation
    pub estimated_gas: u64,
    /// Net profit after gas costs
    pub net_profit: f64,
    /// Source DEX/protocol
    pub source_dex: String,
    /// Target DEX/protocol
    pub target_dex: String,
    /// Price impact percentage
    pub price_impact: f64,
    /// Slippage tolerance
    pub slippage_tolerance: f64,
    /// Requires flash loan
    pub requires_flash_loan: bool,
    /// Flash loan amount if needed
    pub flash_loan_amount: Option<U256>,
    /// Transaction route/path
    pub route: Vec<Address>,
    /// Pool addresses involved
    pub pools: Vec<Address>,
    /// Current block number
    pub block_number: u64,
    /// Opportunity detected timestamp
    pub detected_at: DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Priority score (higher = better)
    pub priority_score: f64,
    /// MEV competition level
    pub competition_level: CompetitionLevel,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Strategy type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrategyType {
    /// S001: Simple DEX Arbitrage
    SimpleArbitrage,
    /// S002: Triangular Arbitrage
    TriangularArbitrage,
    /// S003: Statistical Arbitrage
    StatisticalArbitrage,
    /// S004: JIT Liquidity + Backrun
    JITLiquidityBackrun,
    /// S005: Multi-DEX Arbitrage
    MultiDexArbitrage,
    /// S006: Cross-Chain Arbitrage
    CrossChainArbitrage,
    /// S007: Flash Loan Arbitrage
    FlashLoanArbitrage,
    /// S008: Liquidation MEV
    LiquidationMEV,
    /// S009: Sandwich Attack (Frontrun/Backrun)
    SandwichAttack,
    /// S010: NFT Arbitrage
    NFTArbitrage,
    /// S011: Statistical Z-Score Advanced
    StatisticalZScore,
    /// S012: Governance Arbitrage
    GovernanceArbitrage,
    /// S013: Oracle MEV
    OracleMEV,
    /// S014: Yield Farming Arbitrage
    YieldFarmingArbitrage,
    /// S015: Options Arbitrage
    OptionsArbitrage,
    /// S016: Multi-DEX Triangular
    MultiDexTriangular,
    /// S017: Cross-Chain Bridge Arbitrage
    CrossChainBridge,
    /// S018: NFT Cross-Marketplace
    NFTCrossMarketplace,
    /// S019: DeFi Composite Arbitrage
    DeFiComposite,
    /// S020: Governance + MEV Protection
    GovernanceMEVProtection,
}

impl StrategyType {
    /// Get strategy name as string
    pub fn name(&self) -> &'static str {
        match self {
            Self::SimpleArbitrage => "Simple DEX Arbitrage",
            Self::TriangularArbitrage => "Triangular Arbitrage", 
            Self::StatisticalArbitrage => "Statistical Arbitrage",
            Self::JITLiquidityBackrun => "JIT Liquidity + Backrun",
            Self::MultiDexArbitrage => "Multi-DEX Arbitrage",
            Self::CrossChainArbitrage => "Cross-Chain Arbitrage",
            Self::FlashLoanArbitrage => "Flash Loan Arbitrage",
            Self::LiquidationMEV => "Liquidation MEV",
            Self::SandwichAttack => "Sandwich Attack",
            Self::NFTArbitrage => "NFT Arbitrage",
            Self::StatisticalZScore => "Statistical Z-Score Advanced",
            Self::GovernanceArbitrage => "Governance Arbitrage",
            Self::OracleMEV => "Oracle MEV",
            Self::YieldFarmingArbitrage => "Yield Farming Arbitrage",
            Self::OptionsArbitrage => "Options Arbitrage",
            Self::MultiDexTriangular => "Multi-DEX Triangular",
            Self::CrossChainBridge => "Cross-Chain Bridge Arbitrage",
            Self::NFTCrossMarketplace => "NFT Cross-Marketplace",
            Self::DeFiComposite => "DeFi Composite Arbitrage",
            Self::GovernanceMEVProtection => "Governance + MEV Protection",
        }
    }

    /// Get strategy priority
    pub fn priority(&self) -> u8 {
        match self {
            Self::LiquidationMEV => 10,
            Self::SandwichAttack => 9,
            Self::FlashLoanArbitrage => 8,
            Self::JITLiquidityBackrun => 8,
            Self::StatisticalZScore => 7,
            Self::MultiDexArbitrage => 7,
            Self::CrossChainArbitrage => 6,
            Self::TriangularArbitrage => 6,
            Self::SimpleArbitrage => 5,
            Self::NFTArbitrage => 4,
            Self::GovernanceArbitrage => 4,
            Self::OracleMEV => 5,
            Self::YieldFarmingArbitrage => 3,
            Self::OptionsArbitrage => 3,
            Self::MultiDexTriangular => 6,
            Self::CrossChainBridge => 5,
            Self::NFTCrossMarketplace => 4,
            Self::DeFiComposite => 6,
            Self::GovernanceMEVProtection => 8,
            Self::StatisticalArbitrage => 5,
        }
    }
}

/// MEV competition level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompetitionLevel {
    Low,
    Medium,
    High,
    Extreme,
}

/// DEX information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DEXInfo {
    pub name: String,
    pub router_address: Address,
    pub factory_address: Address,
    pub fee_tier: u32,
    pub supports_flash_swap: bool,
    pub gas_overhead: u64,
}

/// Token pair information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub token0: Address,
    pub token1: Address,
    pub pool_address: Address,
    pub fee: u32,
    pub liquidity: U256,
    pub price: f64,
    pub last_updated: DateTime<Utc>,
}

/// Price quote from DEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuote {
    pub dex_name: String,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub amount_out: U256,
    pub price: f64,
    pub fee: u32,
    pub slippage: f64,
    pub gas_estimate: u64,
    pub timestamp: DateTime<Utc>,
    pub pool_address: Address,
}

/// Flash loan provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanProvider {
    pub name: String,
    pub address: Address,
    pub fee_rate: f64, // Fee as percentage (e.g., 0.09 for 0.09%)
    pub max_loan_amount: U256,
    pub supported_tokens: Vec<Address>,
    pub gas_overhead: u64,
    pub is_active: bool,
}

/// Route information for multi-hop trades
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: Vec<Address>,
    pub pools: Vec<Address>,
    pub fees: Vec<u32>,
    pub estimated_gas: u64,
    pub price_impact: f64,
}

/// Market data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub token_address: Address,
    pub symbol: String,
    pub decimals: u8,
    pub price_usd: f64,
    pub volume_24h: f64,
    pub liquidity_usd: f64,
    pub price_change_24h: f64,
    pub last_updated: DateTime<Utc>,
}

/// Transaction simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub profit: f64,
    pub revert_reason: Option<String>,
    pub state_changes: Vec<StateChange>,
}

/// State change from simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub address: Address,
    pub slot: U256,
    pub before: U256,
    pub after: U256,
}

/// Bundle submission result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSubmissionResult {
    pub bundle_hash: String,
    pub relay_name: String,
    pub submitted_at: DateTime<Utc>,
    pub block_number: u64,
    pub status: BundleStatus,
}

/// Bundle status enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BundleStatus {
    Submitted,
    Included,
    Rejected,
    Failed,
    Expired,
}

/// Risk assessment structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub liquidity_risk: f64,
    pub price_impact_risk: f64,
    pub gas_price_risk: f64,
    pub competition_risk: f64,
    pub overall_risk_score: f64,
    pub risk_level: RiskLevel,
}

/// Risk level enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}