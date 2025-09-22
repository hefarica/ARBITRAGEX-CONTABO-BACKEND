use super::traits::{FlashLender, Quote};
use ethers::types::Address;

pub struct ERC3156GenericLender {
    pub address: Address,
    pub chain_id: u64,
}

impl ERC3156GenericLender {
    pub fn new(address: Address, chain_id: u64) -> Self {
        Self { address, chain_id }
    }
}

#[async_trait::async_trait]
impl FlashLender for ERC3156GenericLender {
    fn name(&self) -> &str { "erc3156_generic" }
    fn chain_id(&self) -> u64 { self.chain_id }
    fn protocol(&self) -> &str { "erc3156" }

    async fn supported_assets(&self) -> Vec<String> {
        vec!["USDC".to_string(), "DAI".to_string()]
    }

    async fn quote(&self, asset: &str, _amount: u128) -> anyhow::Result<Quote> {
        Ok(Quote {
            asset: asset.to_string(),
            amount: 1_000_000_000_000_000,
            fee_bps: 1,
            max_repay: 1_000_000_000_000_000 + (1_000_000_000_000_000 * 1 / 10_000),
        })
    }

    async fn build_tx_payload(&self, _asset: &str, _amount: u128, _swap_data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        Ok(vec![0x01, 0x02, 0x03])
    }
}
