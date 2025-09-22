use super::traits::{FlashLender, Quote};
use ethers::types::Address;

pub struct AaveV3FlashLender {
    pub address: Address,
    pub chain_id: u64,
}

impl AaveV3FlashLender {
    pub fn new(address: Address, chain_id: u64) -> Self {
        Self { address, chain_id }
    }
}

#[async_trait::async_trait]
impl FlashLender for AaveV3FlashLender {
    fn name(&self) -> &str { "aave_v3" }
    fn chain_id(&self) -> u64 { self.chain_id }
    fn protocol(&self) -> &str { "aave_v3" }

    async fn supported_assets(&self) -> Vec<String> {
        vec!["USDC".to_string(), "WETH".to_string(), "DAI".to_string()]
    }

    async fn quote(&self, asset: &str, _amount: u128) -> anyhow::Result<Quote> {
        let fee_bps = match asset {
            "USDC" | "DAI" => 0,
            "WETH" => 2,
            _ => 999,
        };
        Ok(Quote {
            asset: asset.to_string(),
            amount: 1_000_000_000_000_000,
            fee_bps,
            max_repay: 1_000_000_000_000_000 + (1_000_000_000_000_000 * fee_bps as u128 / 10_000),
        })
    }

    async fn build_tx_payload(&self, _asset: &str, _amount: u128, _swap_data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        Ok(vec![0x01, 0x02, 0x03])
    }
}
