use crate::{
    common::AudioFormat,
    sources::playable_track::{PlayableTrack, ResolvedTrack},
};
use async_trait::async_trait;
use std::path::Path;
use tracing::error;

pub struct LocalTrack {
    pub path: String,
}

#[async_trait]
impl PlayableTrack for LocalTrack {
    fn supports_seek(&self) -> bool {
        true
    }

    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let path = self.path.clone();
        let hint = Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .map(AudioFormat::from_ext);
        let reader = tokio::task::spawn_blocking(move || {
            super::reader::LocalFileSource::open(&path)
                .map(|s| Box::new(s) as Box<dyn symphonia::core::io::MediaSource>)
                .map_err(|e| {
                    error!("LocalTrack: failed to open '{path}': {e}");
                    format!("Failed to open file: {e}")
                })
        })
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))??;
        Ok(ResolvedTrack::new(reader, hint))
    }
}
