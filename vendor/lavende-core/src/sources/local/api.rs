use crate::protocol::tracks::TrackInfo;
use std::path::Path;
use symphonia::core::{
    codecs::CODEC_TYPE_NULL,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, StandardTagKey},
    probe::Hint,
};

pub fn probe_file(path: &str) -> Result<TrackInfo, Box<dyn std::error::Error + Send + Sync>> {
    let file = std::fs::File::open(path)?;
    let path_obj = Path::new(path);
    let ext = path_obj
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    let mut hint = Hint::new();
    if let Some(ref e) = ext {
        hint.with_extension(e);
    }
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("no audio track found")?;
    let duration = track
        .codec_params
        .n_frames
        .and_then(|n| {
            track
                .codec_params
                .sample_rate
                .map(|r| (n as f64 / r as f64 * 1000.0) as u64)
        })
        .unwrap_or(0);
    let mut title = String::new();
    let mut author = String::new();
    if let Some(meta) = format.metadata().current() {
        for tag in meta.tags() {
            match tag.std_key {
                Some(StandardTagKey::TrackTitle) => title = tag.value.to_string(),
                Some(StandardTagKey::Artist) | Some(StandardTagKey::AlbumArtist)
                    if author.is_empty() =>
                {
                    author = tag.value.to_string();
                }
                _ => {}
            }
        }
    }
    if title.is_empty() {
        title = path_obj
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_owned();
    }
    if author.is_empty() {
        author = "Unknown Artist".to_owned();
    }
    Ok(TrackInfo {
        identifier: path.to_owned(),
        is_seekable: true,
        author,
        length: duration,
        is_stream: false,
        position: 0,
        title,
        uri: Some(format!("file://{path}")),
        source_name: "local".to_owned(),
        artwork_url: None,
        isrc: None,
    })
}
