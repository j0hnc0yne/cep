use serde::Deserialize;

use crate::error::CepError;

#[derive(Debug, Deserialize)]
pub struct ApiEntry {
    #[serde(rename = "millisUTC")]
    pub millis_utc: String,
    pub price: String,
}

pub fn fetch_current(base_url: &str) -> Result<Vec<ApiEntry>, CepError> {
    let url = format!("{}/api?type=currenthouraverage", base_url);
    fetch(&url)
}

pub fn fetch_range(base_url: &str, start: &str, end: &str) -> Result<Vec<ApiEntry>, CepError> {
    let url = format!(
        "{}/api?type=5minutefeed&datestart={}&dateend={}",
        base_url, start, end
    );
    fetch(&url)
}

fn fetch(url: &str) -> Result<Vec<ApiEntry>, CepError> {
    let client = reqwest::blocking::Client::new();
    let entries: Vec<ApiEntry> = client
        .get(url)
        .send()?
        .error_for_status()
        .map_err(|e| CepError::Api(e.to_string()))?
        .json()?;
    Ok(entries)
}
