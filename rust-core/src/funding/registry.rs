use super::traits::{FlashLender, Quote};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct FundingRegistry {
    providers: Arc<RwLock<HashMap<String, Vec<Box<dyn FlashLender>>>>>,
}

impl FundingRegistry {
    pub fn new() -> Self { Self::default() }

    pub async fn register(&self, chain_id: &str, lender: Box<dyn FlashLender>) {
        let mut map = self.providers.write().await;
        map.entry(chain_id.to_string())
           .or_insert_with(Vec::new)
           .push(lender);
    }

    pub async fn best_lender(
        &self,
        chain_id: &str,
        asset: &str,
        fee_ceiling_bps: u32,
    ) -> Option<(Box<dyn FlashLender>, Quote)> {
        let map = self.providers.read().await;
        let lenders = map.get(chain_id)?;
        for lender in lenders {
            if !lender.supported_assets().await.contains(&asset.to_string()) { continue; }
            match lender.quote(asset, 1_000_000_000_000_000u128).await {
                Ok(quote) if quote.fee_bps <= fee_ceiling_bps => {
                    return Some((lender.clone(), quote));
                },
                _ => continue,
            }
        }
        None
    }
}
