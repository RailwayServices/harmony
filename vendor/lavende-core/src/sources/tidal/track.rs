use super::api::TidalClient;
use crate::{
    audio::source::HttpSource,
    common::types::AudioFormat,
    sources::playable_track::{PlayableTrack, ResolvedTrack},
};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

pub struct TidalTrack {
    pub identifier: String,
    pub stream_url: String,
    pub kind: AudioFormat,
    pub client: Arc<TidalClient>,
}

#[async_trait]
impl PlayableTrack for TidalTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        debug!(
            "TidalTrack: resolving {} with quality {}",
            self.identifier, self.client.quality
        );
        let client_inner = (*self.client.inner).clone();
        let stream_url = self.stream_url.clone();
        let kind = self.kind;
        let reader = HttpSource::new(client_inner, &stream_url)
            .await
            .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
            .map_err(|e| format!("Failed to initialize source: {e}"))?;
        Ok(ResolvedTrack::new(reader, Some(kind)))
    }
}
