use crate::protocol::tracks::TrackInfo;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackInfo {
    pub manifest: String,
    pub manifest_mime_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub urls: Vec<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

#[derive(Clone, Debug)]
pub struct TidalToken {
    pub access_token: String,
    pub expiry_ms: u64,
}

#[derive(Clone, Debug)]
pub enum TidalAuthToken {
    OAuth(String),
    Scraper(String),
}

impl TidalAuthToken {
    pub fn value(&self) -> &str {
        match self {
            Self::OAuth(s) | Self::Scraper(s) => s,
        }
    }
}

pub fn parse_track(item: &Value) -> Option<TrackInfo> {
    let id = item.get("id")?.as_u64()?.to_string();
    let title = item
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Title")
        .to_string();
    let artists = item
        .get("artists")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "Unknown Artist".to_owned());
    let length = item.get("duration").and_then(|v| v.as_u64()).unwrap_or(0) * 1000;
    let isrc = item
        .get("isrc")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned());
    let artwork_url = item
        .get("album")
        .and_then(|a| a.get("cover"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| {
            format!(
                "https://resources.tidal.com/images/{}/1280x1280.jpg",
                s.replace("-", "/")
            )
        });
    let url = item
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.replace("http://", "https://"));

    Some(TrackInfo {
        title,
        author: artists,
        length,
        identifier: id,
        is_stream: false,
        uri: url,
        artwork_url,
        isrc,
        source_name: "tidal".to_owned(),
        is_seekable: true,
        position: 0,
    })
}
