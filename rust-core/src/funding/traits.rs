use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Quote {
    pub asset: String,
    pub amount: u128,
    pub fee_bps: u32,
    pub max_repay: u128,
}

#[async_trait::async_trait]
pub trait FlashLender {
    fn name(&self) -> &str;
    fn chain_id(&self) -> u64;
    fn protocol(&self) -> &str;
    async fn supported_assets(&self) -> Vec<String>;
    async fn quote(&self, asset: &str, amount: u128) -> anyhow::Result<Quote>;
    async fn build_tx_payload(&self, asset: &str, amount: u128, swap_data: Vec<u8>) -> anyhow::Result<Vec<u8>>;
}

#[derive(Clone, Debug)]
pub struct FundingError {
    pub reason: String,
    pub lender: String,
    pub asset: String,
}
