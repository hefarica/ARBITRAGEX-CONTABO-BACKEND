use anyhow::Result;
use ethers::prelude::*;
use uuid::Uuid;

use crate::models::{IssueSeverity, ValidateResponse, ValidationIssue};

pub async fn validate_opportunity(
    opportunity_id: &Uuid,
    strategy_type: &str,
    params: &serde_json::Value,
    provider: &Provider<Http>,
) -> Result<ValidateResponse> {
    let mut issues = Vec::new();
    let mut valid = true;

    // Validate based on strategy type
    match strategy_type {
        "arbitrage" => validate_arbitrage(params, provider, &mut issues).await?,
        "sandwich" => validate_sandwich(params, provider, &mut issues).await?,
        "liquidation" => validate_liquidation(params, provider, &mut issues).await?,
        _ => {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                message: format!("Unknown strategy type: {}", strategy_type),
                field: Some("strategy_type".to_string()),
            });
            valid = false;
        }
    }

    // Estimate profit and gas
    let (profit, gas_estimate) = if valid {
        estimate_profit_and_gas(strategy_type, params, provider).await?
    } else {
        (U256::zero(), U256::zero())
    };

    // Calculate confidence based on issues
    let confidence = calculate_confidence(&issues);

    Ok(ValidateResponse {
        valid,
        profit,
        gas_estimate,
        confidence,
        issues,
    })
}

async fn validate_arbitrage(
    params: &serde_json::Value,
    provider: &Provider<Http>,
    issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
    // Check required parameters
    if params.get("token_in").is_none() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Missing token_in parameter".to_string(),
            field: Some("token_in".to_string()),
        });
    }

    if params.get("token_out").is_none() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Missing token_out parameter".to_string(),
            field: Some("token_out".to_string()),
        });
    }

    if let Some(pools) = params.get("pools").and_then(|p| p.as_array()) {
        if pools.len() < 2 {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                message: "Arbitrage requires at least 2 pools".to_string(),
                field: Some("pools".to_string()),
            });
        }

        // Validate each pool exists
        for (i, pool) in pools.iter().enumerate() {
            if let Some(address) = pool.get("address").and_then(|a| a.as_str()) {
                if let Ok(pool_address) = address.parse::<Address>() {
                    let code = provider.get_code(pool_address, None).await?;
                    if code.is_empty() {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Error,
                            message: format!("Pool {} has no code", address),
                            field: Some(format!("pools[{}].address", i)),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

async fn validate_sandwich(
    params: &serde_json::Value,
    _provider: &Provider<Http>,
    issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
    if params.get("victim_tx").is_none() {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Missing victim transaction".to_string(),
            field: Some("victim_tx".to_string()),
        });
    }

    if let Some(front_run_amount) = params.get("front_run_amount").and_then(|a| a.as_str()) {
        if let Ok(amount) = U256::from_dec_str(front_run_amount) {
            if amount < U256::from(1_000_000_000_000_000_000u64) {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    message: "Front-run amount may be too small".to_string(),
                    field: Some("front_run_amount".to_string()),
                });
            }
        }
    }

    Ok(())
}

async fn validate_liquidation(
    params: &serde_json::Value,
    provider: &Provider<Http>,
    issues: &mut Vec<ValidationIssue>,
) -> Result<()> {
    if let Some(user) = params.get("user").and_then(|u| u.as_str()) {
        if let Ok(user_address) = user.parse::<Address>() {
            // Check if user has any balance
            let balance = provider.get_balance(user_address, None).await?;
            if balance.is_zero() {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    message: "User has zero balance".to_string(),
                    field: Some("user".to_string()),
                });
            }
        }
    } else {
        issues.push(ValidationIssue {
            severity: IssueSeverity::Error,
            message: "Missing user address".to_string(),
            field: Some("user".to_string()),
        });
    }

    Ok(())
}

async fn estimate_profit_and_gas(
    strategy_type: &str,
    params: &serde_json::Value,
    provider: &Provider<Http>,
) -> Result<(U256, U256)> {
    // Simplified estimation - in production would run actual simulation
    let gas_estimate = match strategy_type {
        "arbitrage" => U256::from(300_000),
        "sandwich" => U256::from(600_000),
        "liquidation" => U256::from(500_000),
        _ => U256::from(200_000),
    };

    let gas_price = provider.get_gas_price().await?;
    let gas_cost = gas_estimate * gas_price;

    // Mock profit calculation
    let expected_profit = params
        .get("expected_profit")
        .and_then(|p| p.as_str())
        .and_then(|s| U256::from_dec_str(s).ok())
        .unwrap_or_else(|| U256::from(1_000_000_000_000_000_000u64));

    let net_profit = expected_profit.saturating_sub(gas_cost);

    Ok((net_profit, gas_estimate))
}

fn calculate_confidence(issues: &[ValidationIssue]) -> f64 {
    let errors = issues.iter().filter(|i| matches!(i.severity, IssueSeverity::Error)).count();
    let warnings = issues.iter().filter(|i| matches!(i.severity, IssueSeverity::Warning)).count();

    let base_confidence = 1.0;
    let error_penalty = 0.2;
    let warning_penalty = 0.1;

    (base_confidence - (errors as f64 * error_penalty) - (warnings as f64 * warning_penalty))
        .max(0.0)
        .min(1.0)
}



