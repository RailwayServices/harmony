use crate::{
    protocol::tracks::{LoadResult, Track, TrackInfo},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use regex::Regex;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::sync::{Arc, OnceLock};
use tracing::error;

pub mod extractor;

const SEARCH_URL: &str = "https://www.shazam.com/services/amapi/v1/catalog/US/search";

fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?shazam\.com/song/\d+(?:/[^/?#]+)?")
            .expect("shazam URL regex is a valid literal")
    })
}

pub struct ShazamSource {
    client: Arc<reqwest::Client>,
    search_limit: usize,
}

impl ShazamSource {
    pub fn new(
        config: &crate::config::AppConfig,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        Ok(Self {
            client,
            search_limit: config
                .sources
                .shazam
                .as_ref()
                .map(|c| c.search_limit)
                .unwrap_or(10),
        })
    }

    pub fn base_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36")
    }

    async fn search(&self, query: &str) -> LoadResult {
        let url = format!(
            "{SEARCH_URL}?types=songs&term={query}&limit={limit}",
            query = urlencoding::encode(query),
            limit = self.search_limit
        );
        let resp = match self.base_request(self.client.get(&url)).send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Shazam search request failed: {e}");
                return LoadResult::Empty {};
            }
        };
        if !resp.status().is_success() {
            return LoadResult::Empty {};
        }
        let data: Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse Shazam search JSON: {e}");
                return LoadResult::Empty {};
            }
        };
        let songs = data
            .pointer("/results/songs/data")
            .and_then(|v| v.as_array());
        let Some(songs) = songs else {
            return LoadResult::Empty {};
        };
        let mut tracks = Vec::new();
        for item in songs {
            if let Some(track) = self.build_track(item) {
                tracks.push(track);
            }
        }
        if tracks.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Search(tracks)
        }
    }

    async fn resolve_url(&self, url: &str) -> LoadResult {
        let resp = match self.base_request(self.client.get(url)).send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Shazam resolve request failed: {e}");
                return LoadResult::Empty {};
            }
        };
        if !resp.status().is_success() {
            return LoadResult::Empty {};
        }
        let html = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to read Shazam HTML: {e}");
                return LoadResult::Empty {};
            }
        };
        let title = self.extract_text_after_class(&html, "NewTrackPageHeader_trackTitle__");
        let artist = self.extract_text_after_class(&html, "TrackPageArtistLink_artistNameText__");
        let artwork_url = self.extract_artwork(&html);
        let isrc = self.extract_isrc(&html);
        let duration_ms = self.extract_duration(&html);
        let apple_music_url =
            self.extract_href_starting_with(&html, "https://www.shazam.com/applemusic/song/");
        let mut final_title = title.unwrap_or_else(|| "Unknown".to_owned());
        let mut final_artist = artist.unwrap_or_else(|| "Unknown".to_owned());
        if final_title == "Unknown" {
            self.handle_og_title(&html, &mut final_title, &mut final_artist);
        }
        if final_title == "Unknown" && apple_music_url.is_none() {
            return LoadResult::Empty {};
        }
        let clean_url = url.strip_suffix('/').unwrap_or(url);
        let identifier = clean_url
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .to_owned();
        let mut track = Track::new(TrackInfo {
            identifier,
            is_seekable: true,
            author: final_artist,
            length: duration_ms,
            is_stream: false,
            position: 0,
            title: final_title,
            uri: Some(url.to_owned()),
            artwork_url: artwork_url.or_else(|| self.extract_meta_content(&html, "og:image")),
            isrc,
            source_name: "shazam".to_owned(),
        });
        track.plugin_info = serde_json::json!({
            "albumName": null,
            "albumUrl": null,
            "artistUrl": null,
            "artistArtworkUrl": null,
            "previewUrl": null,
            "isPreview": false
        });
        LoadResult::Track(track)
    }
}

#[async_trait]
impl SourcePlugin for ShazamSource {
    fn name(&self) -> &str {
        "shazam"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || url_regex().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["shsearch:", "szsearch:"]
    }

    fn is_mirror(&self) -> bool {
        true
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
            let query = &identifier[prefix.len()..];
            return self.search(query).await;
        }
        if url_regex().is_match(identifier) {
            return self.resolve_url(identifier).await;
        }
        LoadResult::Empty {}
    }

    async fn get_track(
        &self,
        _identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        None
    }
}
