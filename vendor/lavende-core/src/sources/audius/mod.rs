use crate::{
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub mod api;
pub mod extractor;
pub mod track;

pub struct AudiusSource {
    client: Arc<reqwest::Client>,
    app_name: String,
    search_limit: usize,
    playlist_load_limit: usize,
    album_load_limit: usize,
}

impl AudiusSource {
    pub fn new(
        config: Option<crate::config::AudiusConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let config = config.unwrap_or_default();
        Ok(Self {
            client,
            app_name: config.app_name.unwrap_or_else(|| "Lavende".to_owned()),
            search_limit: config.search_limit,
            playlist_load_limit: config.playlist_load_limit,
            album_load_limit: config.album_load_limit,
        })
    }

    async fn api_request(
        &self,
        endpoint: &str,
        query: Option<std::collections::BTreeMap<String, String>>,
    ) -> Option<Value> {
        api::api_request(&self.client, endpoint, &self.app_name, query).await
    }

    async fn search(&self, query: &str) -> LoadResult {
        let mut params = std::collections::BTreeMap::new();
        params.insert("query".to_owned(), query.to_owned());
        params.insert("limit".to_owned(), self.search_limit.to_string());
        let data = match self.api_request("/v1/tracks/search", Some(params)).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let tracks = extractor::parse_tracks(&data);
        if tracks.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Search(tracks)
        }
    }

    async fn resolve_url(&self, url: &str) -> LoadResult {
        if extractor::playlist_pattern().is_match(url) {
            return self.resolve_playlist_or_album(url, "playlist").await;
        }
        if extractor::album_pattern().is_match(url) {
            return self.resolve_playlist_or_album(url, "album").await;
        }
        if extractor::track_pattern().is_match(url) {
            return self.resolve_track(url).await;
        }
        if extractor::user_pattern().is_match(url) {
            return self.resolve_user(url).await;
        }
        LoadResult::Empty {}
    }

    async fn resolve_track(&self, url: &str) -> LoadResult {
        let mut params = std::collections::BTreeMap::new();
        params.insert("url".to_owned(), url.to_owned());
        let data = match self.api_request("/v1/resolve", Some(params)).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        match extractor::build_track(&data) {
            Some(t) => LoadResult::Track(t),
            None => LoadResult::Empty {},
        }
    }

    async fn resolve_playlist_or_album(&self, url: &str, type_: &str) -> LoadResult {
        let mut params = std::collections::BTreeMap::new();
        params.insert("url".to_owned(), url.to_owned());
        let data = match self.api_request("/v1/resolve", Some(params)).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let id = match data["id"].as_str() {
            Some(i) => i,
            None => return LoadResult::Empty {},
        };
        let limit = if type_ == "playlist" {
            self.playlist_load_limit
        } else {
            self.album_load_limit
        };
        let mut tracks_params = std::collections::BTreeMap::new();
        tracks_params.insert("limit".to_owned(), limit.to_string());
        let tracks_data = match self
            .api_request(&format!("/v1/playlists/{id}/tracks"), Some(tracks_params))
            .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let tracks = extractor::parse_tracks(&tracks_data);
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        let name = data["playlist_name"]
            .as_str()
            .unwrap_or("Audius Playlist")
            .to_owned();
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name,
                selected_track: -1,
            },
            plugin_info: serde_json::json!({}),
            tracks,
        })
    }

    async fn resolve_user(&self, url: &str) -> LoadResult {
        let mut params = std::collections::BTreeMap::new();
        params.insert("url".to_owned(), url.to_owned());
        let data = match self.api_request("/v1/resolve", Some(params)).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let id = match data["id"].as_str() {
            Some(i) => i,
            None => return LoadResult::Empty {},
        };
        let mut tracks_params = std::collections::BTreeMap::new();
        tracks_params.insert("limit".to_owned(), self.search_limit.to_string());
        let tracks_data = match self
            .api_request(&format!("/v1/users/{id}/tracks"), Some(tracks_params))
            .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let tracks = extractor::parse_tracks(&tracks_data);
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        let name = format!("{}'s Tracks", data["name"].as_str().unwrap_or("Artist"));
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name,
                selected_track: -1,
            },
            plugin_info: serde_json::json!({}),
            tracks,
        })
    }
}

#[async_trait]
impl SourcePlugin for AudiusSource {
    fn name(&self) -> &str {
        "audius"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || extractor::track_pattern().is_match(identifier)
            || extractor::playlist_pattern().is_match(identifier)
            || extractor::album_pattern().is_match(identifier)
            || extractor::user_pattern().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["ausearch:", "audsearch:"]
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
        self.resolve_url(identifier).await
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let track_id = if identifier.starts_with("http") {
            let mut params = std::collections::BTreeMap::new();
            params.insert("url".to_owned(), identifier.to_owned());
            let data = self.api_request("/v1/resolve", Some(params)).await?;
            data["id"].as_str()?.to_owned()
        } else {
            identifier.to_owned()
        };
        let stream_url = api::fetch_stream_url(&self.client, &track_id, &self.app_name).await?;
        Some(Arc::new(track::AudiusTrack {
            client: self.client.clone(),
            track_id,
            stream_url: Some(stream_url),
            app_name: self.app_name.clone(),
            local_addr: routeplanner.and_then(|rp| rp.get_address()),
        }))
    }
}
