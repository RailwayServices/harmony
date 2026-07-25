use crate::sources::playable_track::{PlayableTrack, ResolvedTrack};
use async_trait::async_trait;
use std::sync::Arc;

pub struct AudiusTrack {
    pub client: Arc<reqwest::Client>,
    pub track_id: String,
    pub stream_url: Option<String>,
    pub app_name: String,
    pub local_addr: Option<std::net::IpAddr>,
}

#[async_trait]
impl PlayableTrack for AudiusTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let url = if let Some(url) = self.stream_url.clone() {
            url
        } else {
            super::api::fetch_stream_url(&self.client, &self.track_id, &self.app_name)
                .await
                .ok_or_else(|| {
                    format!(
                        "Failed to fetch Audius stream URL for track ID {}",
                        self.track_id
                    )
                })?
        };

        crate::sources::http::HttpTrack {
            url,
            local_addr: self.local_addr,
            proxy: None,
        }
        .resolve()
        .await
    }
}
