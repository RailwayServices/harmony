use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

pub const API_URL: &str = "https://gaana.com/apiv2";
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

pub fn base_request(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Origin", "https://gaana.com")
        .header("Connection", "keep-alive")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "same-origin")
        .header(
            "sec-ch-ua",
            "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"",
        )
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
}

pub async fn get_json(
    client: &Arc<reqwest::Client>,
    params: &[(&str, &str)],
    referer_path: &str,
) -> Option<Value> {
    let url = format!(
        "{}?{}",
        API_URL,
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    );
    let resp = match base_request(client.post(&url))
        .header("Referer", format!("https://gaana.com/{}", referer_path))
        .header("Content-Length", "0")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("Gaana API request failed: {}", e);
            return None;
        }
    };
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    serde_json::from_str::<Value>(&text).ok()
}

pub async fn fetch_stream_url_internal(
    client: &Arc<reqwest::Client>,
    track_id: &str,
    quality: &str,
) -> Option<String> {
    let body = format!(
        "quality={}&track_id={}&stream_format=mp4",
        urlencoding::encode(quality),
        urlencoding::encode(track_id)
    );
    let resp = client
        .post("https://gaana.com/api/stream-url")
        .header("User-Agent", USER_AGENT)
        .header("Referer", "https://gaana.com/")
        .header("Origin", "https://gaana.com")
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: serde_json::Value = resp.json().await.ok()?;
    let encrypted_path = data.get("data")?.get("stream_path")?.as_str()?;
    super::crypto::decrypt_stream_path(encrypted_path)
}
