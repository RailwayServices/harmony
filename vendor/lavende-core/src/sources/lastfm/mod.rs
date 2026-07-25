use crate::{
    protocol::tracks::LoadResult,
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use std::sync::Arc;

pub mod api;
pub mod extractor;

pub struct LastFMSource {
    pub http: Arc<reqwest::Client>,
    pub api_key: Option<String>,
    pub search_limit: usize,
}

impl LastFMSource {
    pub fn new(
        config: Option<crate::config::sources::LastFmConfig>,
        http: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (api_key, search_limit) = if let Some(c) = config {
            (c.api_key, c.search_limit)
        } else {
            (None, 10)
        };
        Ok(Self {
            http,
            api_key,
            search_limit,
        })
    }
}

#[async_trait]
impl SourcePlugin for LastFMSource {
    fn name(&self) -> &str {
        "lastfm"
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["lfsearch:", "lfmsearch:"]
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || api::path_regex().is_match(identifier)
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
            self.search_tracks(query).await
        } else {
            self.resolve_url(identifier).await
        }
    }

    async fn get_track(
        &self,
        _identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        None
    }
}
