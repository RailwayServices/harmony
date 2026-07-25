use crate::protocol::tracks::{Track, TrackInfo};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

pub fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?:https?://)?(?:www\.)?gaana\.com/(?P<type>song|album|playlist|artist)/(?P<seokey>[\w-]+)",
        )
        .unwrap()
    })
}

pub fn extract_isrc(json: &Value) -> Option<String> {
    if let Some(isrc) = json.get("isrc").and_then(|v| v.as_str()) {
        return Some(isrc.to_owned());
    }
    if let Some(info) = json.get("entity_info").and_then(|v| v.as_array()) {
        return info.iter().find_map(|e| {
            if e.get("key").and_then(|k| k.as_str()) == Some("isrc") {
                e.get("value")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned())
            } else {
                None
            }
        });
    }
    None
}

pub fn parse_track(json: &Value) -> Option<Track> {
    let id = json
        .get("track_id")
        .and_then(|v| {
            v.as_str().map(|s| s.to_owned()).or_else(|| {
                v.as_i64()
                    .map(|i| i.to_string())
                    .or_else(|| v.as_u64().map(|i| i.to_string()))
            })
        })
        .or_else(|| {
            json.get("entity_id").and_then(|v| {
                v.as_str()
                    .map(|s| s.to_owned())
                    .or_else(|| v.as_i64().map(|i| i.to_string()))
            })
        })?;
    let title = json
        .get("track_title")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("name").and_then(|v| v.as_str()))?;
    let duration = json
        .get("duration")
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0)
        * 1000;
    let author = if let Some(artist_arr) = json.get("artist").and_then(|v| v.as_array()) {
        let names: Vec<&str> = artist_arr
            .iter()
            .filter_map(|a| a.get("name").and_then(|v| v.as_str()))
            .collect();
        if names.is_empty() {
            "Unknown Artist".to_owned()
        } else {
            names.join(", ")
        }
    } else {
        "Unknown Artist".to_owned()
    };
    let seokey = json.get("seokey").and_then(|v| v.as_str());
    let uri = seokey.map(|s| format!("https://gaana.com/song/{s}"));
    let artwork_url = json
        .get("artwork_large")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            json.get("atw")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
        })
        .map(|s| s.to_owned());
    let isrc = extract_isrc(json);
    let track_info = TrackInfo {
        identifier: id,
        is_seekable: true,
        author,
        length: duration,
        is_stream: false,
        position: 0,
        title: title.to_owned(),
        uri,
        artwork_url,
        isrc,
        source_name: "gaana".to_owned(),
    };
    Some(Track::new(track_info))
}

pub fn parse_entity_track(json: &Value) -> Option<Track> {
    let id = json.get("entity_id").and_then(|v| {
        v.as_str()
            .map(|s| s.to_owned())
            .or_else(|| v.as_i64().map(|i| i.to_string()))
    })?;
    let title = json.get("name").and_then(|v| v.as_str())?;
    let entity_info = json.get("entity_info").and_then(|v| v.as_array());
    let get_entity_value = |key: &str| -> Option<&Value> {
        entity_info?.iter().find_map(|e| {
            if e.get("key").and_then(|k| k.as_str()) == Some(key) {
                e.get("value")
            } else {
                None
            }
        })
    };
    let duration = get_entity_value("duration")
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0)
        * 1000;
    let author = get_entity_value("artist")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "Unknown Artist".to_owned());
    let seokey = json.get("seokey").and_then(|v| v.as_str());
    let uri = seokey.map(|s| format!("https://gaana.com/song/{s}"));
    let artwork_url = json
        .get("atw")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let isrc = extract_isrc(json);
    let track_info = TrackInfo {
        identifier: id,
        is_seekable: true,
        author,
        length: duration,
        is_stream: false,
        position: 0,
        title: title.to_owned(),
        uri,
        artwork_url,
        isrc,
        source_name: "gaana".to_owned(),
    };
    Some(Track::new(track_info))
}
