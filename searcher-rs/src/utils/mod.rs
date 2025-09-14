use ethers::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn format_wei_to_ether(wei: U256) -> String {
    ethers::utils::format_ether(wei)
}

pub fn calculate_price_impact(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
) -> f64 {
    let amount_in_f64 = amount_in.as_u64() as f64;
    let reserve_in_f64 = reserve_in.as_u64() as f64;
    
    // Price impact = (amount_in / (reserve_in + amount_in)) * 100
    (amount_in_f64 / (reserve_in_f64 + amount_in_f64)) * 100.0
}

pub fn is_profitable_after_gas(
    gross_profit: U256,
    gas_cost: U256,
    min_profit: U256,
) -> bool {
    gross_profit > gas_cost + min_profit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_impact() {
        let amount = U256::from(1_000_000);
        let reserve_in = U256::from(100_000_000);
        let reserve_out = U256::from(100_000_000);
        
        let impact = calculate_price_impact(amount, reserve_in, reserve_out);
        assert!(impact > 0.0 && impact < 2.0);
    }

    #[test]
    fn test_profitable_check() {
        let profit = U256::from(1_000_000);
        let gas = U256::from(300_000);
        let min = U256::from(100_000);
        
        assert!(is_profitable_after_gas(profit, gas, min));
        assert!(!is_profitable_after_gas(gas, profit, min));
    }
}

