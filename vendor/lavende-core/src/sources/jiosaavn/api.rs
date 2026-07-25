use crate::common::types::AnyResult;
use serde_json::Value;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";

pub async fn get_json(
    client: &reqwest::Client,
    api_url: &str,
    params: &[(&str, &str)],
) -> AnyResult<Value> {
    let resp = client
        .get(api_url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Referer", "https://www.jiosaavn.com/")
        .header("Origin", "https://www.jiosaavn.com")
        .query(params)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("JioSaavn API status: {}", resp.status()).into());
    }

    let text = resp.text().await?;
    let val: Value = serde_json::from_str(&text)?;
    Ok(val)
}
