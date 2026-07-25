use crate::protocol::tracks::{Track, TrackInfo};
use base64::{Engine as _, engine::general_purpose};
use serde_json::Value;

const DECRYPTION_KEY: &[u8] = b"IFYOUWANTTHEARTISTSTOGETPAIDDONOTDOWNLOADFROMMIXCLOUD";

pub fn decrypt(ciphertext_b64: &str) -> String {
    let ciphertext: Vec<u8> = match general_purpose::STANDARD.decode(ciphertext_b64) {
        Ok(b) => b,
        Err(_) => return String::new(),
    };
    let mut decrypted = Vec::with_capacity(ciphertext.len());
    for (i, &byte) in ciphertext.iter().enumerate() {
        decrypted.push(byte ^ DECRYPTION_KEY[i % DECRYPTION_KEY.len()]);
    }
    String::from_utf8(decrypted).unwrap_or_default()
}

pub fn parse_track_data(data: &Value) -> Option<Track> {
    let url_raw = data["url"].as_str()?;
    let path_parts: Vec<&str> = url_raw
        .split("mixcloud.com/")
        .nth(1)?
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if path_parts.len() < 2 {
        return None;
    }
    let user_id = path_parts[0];
    let slug = path_parts[1];
    let id = format!("{user_id}_{slug}");
    let title = data["name"].as_str()?.to_owned();
    let author = data["owner"]["displayName"]
        .as_str()
        .unwrap_or(user_id)
        .to_owned();
    let duration_ms = data["audioLength"].as_u64().unwrap_or(0) * 1000;
    let artwork_url = data["picture"]["url"].as_str().map(|s| s.to_owned());

    Some(Track::new(TrackInfo {
        identifier: id,
        is_seekable: true,
        author,
        length: duration_ms,
        is_stream: false,
        position: 0,
        title,
        uri: Some(url_raw.to_owned()),
        artwork_url,
        isrc: None,
        source_name: "mixcloud".to_owned(),
    }))
}
