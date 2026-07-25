use crate::{
    common::types::AudioFormat,
    sources::{
        mixcloud::{api, extractor},
        playable_track::{PlayableTrack, ResolvedTrack},
    },
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct MixcloudTrack {
    pub client: Arc<reqwest::Client>,
    pub hls_url: Option<String>,
    pub stream_url: Option<String>,
    pub uri: String,
    pub local_addr: Option<std::net::IpAddr>,
}

#[async_trait]
impl PlayableTrack for MixcloudTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let (hls_url, stream_url) = if self.hls_url.is_some() || self.stream_url.is_some() {
            (self.hls_url.clone(), self.stream_url.clone())
        } else {
            let (enc_hls, enc_url) = api::fetch_track_stream_info(&self.client, &self.uri)
                .await
                .unwrap_or((None, None));
            (
                enc_hls.map(|s| extractor::decrypt(&s)),
                enc_url.map(|s| extractor::decrypt(&s)),
            )
        };

        let local_addr = self.local_addr;
        let uri = self.uri.clone();

        if let Some(url) = hls_url {
            crate::sources::youtube::hls::HlsReader::new(&url, local_addr, None, None, None)
                .await
                .map(|r| {
                    ResolvedTrack::new(
                        Box::new(r) as Box<dyn symphonia::core::io::MediaSource>,
                        Some(AudioFormat::Aac),
                    )
                })
                .map_err(|e| format!("Failed to init HLS reader: {e}"))
        } else if let Some(url) = stream_url {
            let hint = std::path::Path::new(&url)
                .extension()
                .and_then(|s| s.to_str())
                .map(AudioFormat::from_ext)
                .or(Some(AudioFormat::Mp4));

            super::reader::MixcloudReader::new(&url, local_addr)
                .await
                .map(|r| {
                    ResolvedTrack::new(
                        Box::new(r) as Box<dyn symphonia::core::io::MediaSource>,
                        hint,
                    )
                })
                .map_err(|e| format!("Failed to init reader: {e}"))
        } else {
            Err(format!("No stream URL available for {uri}"))
        }
    }
}
