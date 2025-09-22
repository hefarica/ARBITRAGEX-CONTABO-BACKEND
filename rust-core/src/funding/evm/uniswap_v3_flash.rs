use super::traits::{FlashLender, Quote};
use ethers::types::Address;

pub struct UniswapV3FlashLender {
    pub pool_address: Address,
    pub chain_id: u64,
}

impl UniswapV3FlashLender {
    pub fn new(pool_address: Address, chain_id: u64) -> Self {
        Self { pool_address, chain_id }
    }
}

#[async_trait::async_trait]
impl FlashLender for UniswapV3FlashLender {
    fn name(&self) -> &str { "uniswap_v3_flash" }
    fn chain_id(&self) -> u64 { self.chain_id }
    fn protocol(&self) -> &str { "uniswap_v3" }

    async fn supported_assets(&self) -> Vec<String> {
        vec!["USDC".to_string(), "WETH".to_string()]
    }

    async fn quote(&self, asset: &str, _amount: u128) -> anyhow::Result<Quote> {
        let fee_bps = 5;
        Ok(Quote {
            asset: asset.to_string(),
            amount: 1_000_000_000_000_000,
            fee_bps,
            max_repay: 1_000_000_000_000_000 + (1_000_000_000_000_000 * fee_bps as u128 / 10_000),
        })
    }

    async fn build_tx_payload(&self, asset: &str, amount: u128, swap_data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // Simular datos de swap
        Ok(payload)
    }
}
