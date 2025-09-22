use reqwest::Client;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

pub async fn fetch_real<T: DeserializeOwned + Validatable>(url: &str) -> anyhow::Result<T> {
    let resp = Client::new().get(url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} for {}", resp.status(), url);
    }
    let val: T = resp.json().await?;
    if !val.is_valid() {
        anyhow::bail!("Invalid data from {}", url);
    }
    Ok(val)
}

pub trait Validatable {
    fn is_valid(&self) -> bool;
}
