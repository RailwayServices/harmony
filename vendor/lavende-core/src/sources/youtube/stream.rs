use crate::{
    audio::source::{SegmentedSource, create_client},
    common::types::AnyResult,
    config::HttpProxyConfig,
    sources::{
        http::reader::HttpReader,
        youtube::{cipher::YouTubeCipherManager, hls::HlsReader, identity::cdn_user_agent},
    },
};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use symphonia::core::io::MediaSource;

pub struct YoutubeReader {
    inner: SegmentedSource,
}

impl YoutubeReader {
    pub async fn new(
        url: &str,
        local_addr: Option<std::net::IpAddr>,
        proxy: Option<HttpProxyConfig>,
    ) -> AnyResult<Self> {
        let user_agent = cdn_user_agent(url)
            .map(str::to_string)
            .unwrap_or_else(crate::common::utils::default_user_agent);
        let client = create_client(user_agent, local_addr, proxy, None)?;
        let inner = SegmentedSource::new(client, url).await?;
        Ok(Self { inner })
    }
}

impl Read for YoutubeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for YoutubeReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl MediaSource for YoutubeReader {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.inner.byte_len()
    }
}

pub async fn create_reader(
    url: &str,
    client_name: &str,
    local_addr: Option<std::net::IpAddr>,
    proxy: Option<HttpProxyConfig>,
    cipher_manager: Arc<YouTubeCipherManager>,
) -> AnyResult<Box<dyn MediaSource>> {
    if url.contains(".m3u8") || url.contains("/playlist") {
        Ok(Box::new(
            HlsReader::new(url, local_addr, Some(cipher_manager), None, proxy).await?,
        ))
    } else if client_name == "TV" {
        Ok(Box::new(YoutubeReader::new(url, local_addr, proxy).await?))
    } else {
        Ok(Box::new(HttpReader::new(url, local_addr, proxy).await?))
    }
}
