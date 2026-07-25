use crate::{
    protocol::tracks::{LoadResult, Track, TrackInfo},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

pub mod api;
pub mod extractor;
pub mod reader;
pub mod token;
pub mod track;

pub fn decrypt(ciphertext_b64: &str) -> String {
    extractor::decrypt(ciphertext_b64)
}

fn track_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^https?://(?:(?:www|beta|m)\.)?mixcloud\.com/(?P<user>[^/]+)/(?P<slug>[^/]+)/?$",
        )
        .unwrap()
    })
}

fn playlist_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^https?://(?:(?:www|beta|m)\.)?mixcloud\.com/(?P<user>[^/]+)/playlists/(?P<playlist>[^/]+)/?$").unwrap()
    })
}

fn user_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^https?://(?:(?:www|beta|m)\.)?mixcloud\.com/(?P<id>[^/]+)(?:/(?P<type>uploads|favorites|listens|stream))?/?$").unwrap()
    })
}

pub struct MixcloudSource {
    client: Arc<reqwest::Client>,
    search_limit: usize,
}

impl MixcloudSource {
    pub fn new(
        config: Option<crate::config::MixcloudConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        Ok(Self {
            client,
            search_limit: config.map(|c| c.search_limit).unwrap_or(10),
        })
    }

    pub fn parse_track_data(&self, data: &Value) -> Option<Track> {
        extractor::parse_track_data(data)
    }

    async fn search(&self, query_raw: &str) -> LoadResult {
        let url = format!(
            "https://api.mixcloud.com/search/?q={}&type=cloudcast",
            urlencoding::encode(query_raw)
        );
        let resp = match self.client.get(url).send().await {
            Ok(r) => r,
            Err(_) => return LoadResult::Empty {},
        };
        let body: Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return LoadResult::Empty {},
        };

        let mut tracks = Vec::new();
        if let Some(data) = body["data"].as_array() {
            for item in data {
                if let Some(url_raw) = item["url"].as_str() {
                    let path_parts: Vec<&str> = url_raw
                        .split("mixcloud.com/")
                        .nth(1)
                        .unwrap_or("")
                        .split('/')
                        .filter(|s| !s.is_empty())
                        .collect();
                    if path_parts.len() < 2 {
                        continue;
                    }
                    let id = format!("{}_{}", path_parts[0], path_parts[1]);
                    tracks.push(Track::new(TrackInfo {
                        identifier: id,
                        is_seekable: true,
                        author: item["user"]["name"]
                            .as_str()
                            .unwrap_or(path_parts[0])
                            .to_owned(),
                        length: item["audio_length"].as_u64().unwrap_or(0) * 1000,
                        is_stream: false,
                        position: 0,
                        title: item["name"].as_str().unwrap_or("Unknown").to_owned(),
                        uri: Some(url_raw.to_owned()),
                        artwork_url: item["pictures"]["large"]
                            .as_str()
                            .or_else(|| item["pictures"]["medium"].as_str())
                            .map(|s| s.to_owned()),
                        isrc: None,
                        source_name: "mixcloud".to_owned(),
                    }));
                }
                if tracks.len() >= self.search_limit {
                    break;
                }
            }
        }

        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Search(tracks)
    }
}

#[async_trait]
impl SourcePlugin for MixcloudSource {
    fn name(&self) -> &str {
        "mixcloud"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || track_url_re().is_match(identifier)
            || playlist_url_re().is_match(identifier)
            || user_url_re().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["mcsearch:"]
    }

    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        if let Some(prefix) = self
            .search_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            return self.search(&identifier[prefix.len()..]).await;
        }

        if let Some(caps) = playlist_url_re().captures(identifier) {
            return api::resolve_playlist(self, &caps["user"], &caps["playlist"]).await;
        }

        if let Some(caps) = user_url_re().captures(identifier) {
            return api::resolve_user(
                self,
                &caps["id"],
                caps.name("type").map(|m| m.as_str()).unwrap_or("uploads"),
            )
            .await;
        }

        if let Some(caps) = track_url_re().captures(identifier) {
            return api::resolve_track(self, &caps["user"], &caps["slug"]).await;
        }

        LoadResult::Empty {}
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let url = match self.load(identifier, None).await {
            LoadResult::Track(track) => track.info.uri?,
            _ => return None,
        };

        let (enc_hls, enc_url) = api::fetch_track_stream_info(&self.client, &url)
            .await
            .unwrap_or((None, None));

        let hls_url = enc_hls.map(|s| decrypt(&s));
        let stream_url = enc_url.map(|s| decrypt(&s));

        if hls_url.is_none() && stream_url.is_none() {
            return None;
        }

        Some(Arc::new(track::MixcloudTrack {
            client: self.client.clone(),
            hls_url,
            stream_url,
            uri: url,
            local_addr: routeplanner.and_then(|rp| rp.get_address()),
        }))
    }
}
