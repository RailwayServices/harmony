use serde_json::Value;
use std::sync::Arc;

pub const API_BASE: &str = "https://discoveryprovider.audius.co";

pub async fn fetch_stream_url(
    client: &Arc<reqwest::Client>,
    track_id: &str,
    app_name: &str,
) -> Option<String> {
    let url = format!(
        "{API_BASE}/v1/tracks/{}/stream",
        urlencoding::encode(track_id)
    );
    let resp = client
        .get(url)
        .query(&[("app_name", app_name), ("no_redirect", "true")])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    body["data"].as_str().map(|s| s.to_owned())
}

pub async fn api_request(
    client: &Arc<reqwest::Client>,
    endpoint: &str,
    app_name: &str,
    query: Option<std::collections::BTreeMap<String, String>>,
) -> Option<Value> {
    let url = format!("{API_BASE}{endpoint}");
    let mut builder = client.get(&url);
    if let Some(q) = query {
        builder = builder.query(&q);
    }
    builder = builder.query(&[("app_name", app_name)]);
    let resp = builder.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: Value = resp.json().await.ok()?;
    Some(body["data"].clone())
}
