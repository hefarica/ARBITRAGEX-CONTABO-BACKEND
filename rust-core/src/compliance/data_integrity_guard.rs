use crate::compliance::fetch_real;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct DataIntegrityRecord {
    pub url: String,
    pub status: u16,
    pub timestamp: SystemTime,
    pub checksum: String,
    pub source: &'static str,
}

pub async fn audit_fetch<T: serde::Serialize + Validatable>(
    url: &str, source: &'static str
) -> anyhow::Result<(T, DataIntegrityRecord)> {
    let start = SystemTime::now();
    let data = fetch_real::<T>(url).await?;
    let json_str = serde_json::to_string(&data)?;
    let checksum = sha2::Sha256::digest(json_str.as_bytes()).to_vec();
    let record = DataIntegrityRecord {
        url: url.to_string(),
        status: 200,
        timestamp: start,
        checksum: format!("{:x}", checksum),
        source,
    };
    Ok((data, record))
}
