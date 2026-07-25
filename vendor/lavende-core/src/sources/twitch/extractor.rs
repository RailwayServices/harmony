use crate::sources::youtube::hls::{types::Resource, utils::resolve_url};

pub struct ChannelStreamInfo {
    pub quality: String,
    pub url: String,
}

pub fn load_channel_streams_list(m3u8: &str) -> Vec<ChannelStreamInfo> {
    let lines: Vec<&str> = m3u8.lines().collect();
    let mut streams = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("#EXT-X-STREAM-INF:") {
            let quality = line
                .split(',')
                .find_map(|part| {
                    part.trim()
                        .strip_prefix("VIDEO=\"")
                        .and_then(|v| v.strip_suffix('"'))
                })
                .unwrap_or("unknown")
                .to_string();
            if let Some(url_line) = lines.get(i + 1) {
                let url = url_line.trim();
                if !url.is_empty() && !url.starts_with('#') {
                    streams.push(ChannelStreamInfo {
                        quality,
                        url: url.to_string(),
                    });
                }
            }
        }
        i += 1;
    }
    streams
}

pub fn parse_live_playlist(text: &str, base_url: &str) -> (Vec<Resource>, f64) {
    let mut segments = Vec::new();
    let mut target_duration = 6.0f64;
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(rest) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            if let Ok(d) = rest.trim().parse::<f64>() {
                target_duration = d;
            }
        } else if line.starts_with("#EXTINF:") {
            let duration = line
                .strip_prefix("#EXTINF:")
                .and_then(|r| r.split(',').next())
                .and_then(|d| d.trim().parse::<f64>().ok());
            let mut j = i + 1;
            while j < lines.len() && lines[j].starts_with('#') {
                j += 1;
            }
            if j < lines.len() && !lines[j].is_empty() {
                let url = resolve_url(base_url, lines[j]);
                segments.push(Resource {
                    url,
                    range: None,
                    duration,
                });
            }
            i = j + 1;
            continue;
        }
        i += 1;
    }
    (segments, target_duration)
}
