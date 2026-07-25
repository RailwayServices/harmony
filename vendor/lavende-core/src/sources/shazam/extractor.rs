use crate::protocol::tracks::{Track, TrackInfo};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

fn og_title_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^(.+?) - (.+?):").expect("shazam og:title regex is a valid literal")
    })
}

impl super::ShazamSource {
    pub fn build_track(&self, item: &Value) -> Option<Track> {
        let attributes = item.get("attributes")?;
        let id = item.get("id")?.as_str()?.to_owned();
        let title = attributes
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_owned();
        let author = attributes
            .get("artistName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_owned();
        let length = attributes
            .get("durationInMillis")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let isrc = attributes
            .get("isrc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let artwork_url = attributes
            .get("artwork")
            .and_then(|a| a.get("url"))
            .and_then(|v| v.as_str())
            .map(|s| s.replace("{w}", "1000").replace("{h}", "1000"));
        let uri = attributes
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let mut track = Track::new(TrackInfo {
            identifier: id,
            is_seekable: true,
            author,
            length,
            is_stream: false,
            position: 0,
            title,
            uri: uri.clone(),
            artwork_url,
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
        Some(track)
    }

    pub fn extract_text_after_class(&self, html: &str, class_part: &str) -> Option<String> {
        let mut from = 0;
        while let Some(c) = html[from..].find("class=\"") {
            let c = from + c;
            let q = html[c + 7..].find('"').map(|i| c + 7 + i)?;
            let cls = &html[c + 7..q];
            if cls.contains(class_part) {
                let gt = html[q..].find('>').map(|i| q + i)?;
                let lt = html[gt + 1..].find('<').map(|i| gt + 1 + i)?;
                let text = html[gt + 1..lt].trim().to_owned();
                if !text.is_empty() {
                    return Some(text);
                }
            }
            from = q + 1;
        }
        None
    }

    pub fn extract_href_starting_with(&self, html: &str, prefix: &str) -> Option<String> {
        let pattern = format!("href=\"{prefix}\"");
        if let Some(i) = html.find(&pattern) {
            let start = i + 6;
            if let Some(end) = html[start..].find('"') {
                return Some(html[start..start + end].to_owned());
            }
        }
        None
    }

    pub fn extract_artwork(&self, html: &str) -> Option<String> {
        if let Some(og) = self.extract_meta_content(html, "og:image") {
            return Some(og);
        }
        let mut alt_idx = html.find("alt=\"album cover\"");
        if alt_idx.is_none() {
            alt_idx = html.find("alt=\"song thumbnail\"");
        }
        let alt_idx = alt_idx?;
        let img_start = html[..alt_idx].rfind("<img")?;
        let img_end = html[alt_idx..].find('>')? + alt_idx;
        let tag = &html[img_start..img_end + 1];
        if let Some(s) = tag.find("srcset=\"") {
            let val_start = s + 8;
            if let Some(val_end) = tag[val_start..].find('"') {
                let srcset = &tag[val_start..val_start + val_end];
                let space = srcset.find(' ').unwrap_or(srcset.len());
                return Some(srcset[..space].to_owned());
            }
        }
        None
    }

    pub fn extract_isrc(&self, html: &str) -> Option<String> {
        let tokens = ["\"isrc\"", "\\\"isrc\\\""];
        for token in tokens {
            let mut from = 0;
            while let Some(at) = html[from..].find(token) {
                let at = from + at;
                from = at + token.len();
                let mut i = match html[from..].find(':') {
                    Some(idx) => from + idx + 1,
                    None => continue,
                };
                while i < html.len() {
                    let bytes = html.as_bytes();
                    if bytes[i] != b' '
                        && bytes[i] != b'\t'
                        && bytes[i] != b'\n'
                        && bytes[i] != b'\r'
                    {
                        break;
                    }
                    i += 1;
                }
                while i < html.len() && html.as_bytes()[i] == b'\\' {
                    i += 1;
                }
                if i >= html.len() || html.as_bytes()[i] != b'"' {
                    continue;
                }
                i += 1;
                if i + 12 > html.len() {
                    continue;
                }
                let isrc_cand = &html[i..i + 12];
                if self.is_valid_isrc(isrc_cand) {
                    return Some(isrc_cand.to_owned());
                }
            }
        }
        None
    }

    fn is_valid_isrc(&self, s: &str) -> bool {
        if s.len() != 12 {
            return false;
        }
        let b = s.as_bytes();
        if !b[0].is_ascii_uppercase() || !b[1].is_ascii_uppercase() {
            return false;
        }
        for &c in &b[2..5] {
            if !c.is_ascii_uppercase() && !c.is_ascii_digit() {
                return false;
            }
        }
        for &c in &b[5..12] {
            if !c.is_ascii_digit() {
                return false;
            }
        }
        true
    }

    pub fn extract_duration(&self, html: &str) -> u64 {
        let needles = [
            "\"duration\":\"PT",
            "\"duration\": \"PT",
            "\\\"duration\\\":\\\"PT",
        ];
        for n in needles {
            if let Some(at) = html.find(n) {
                let start = at + n.len() - 2;
                let bracket = n.starts_with('\\');
                let end_char = if bracket { '\\' } else { '"' };
                if let Some(end) = html[start..].find(end_char) {
                    return self.parse_iso_duration(&html[start..start + end]);
                }
            }
        }
        0
    }

    fn parse_iso_duration(&self, iso: &str) -> u64 {
        if !iso.contains('T') {
            return 0;
        }
        let t_idx = iso.find('T').unwrap() + 1;
        let mut ms = 0.0;
        let mut current_num = String::new();
        for c in iso[t_idx..].chars() {
            if c.is_ascii_digit() || c == '.' {
                current_num.push(c);
            } else {
                let val = current_num.parse::<f64>().unwrap_or(0.0);
                match c {
                    'H' => ms += val * 3600000.0,
                    'M' => ms += val * 60000.0,
                    'S' => ms += val * 1000.0,
                    _ => break,
                }
                current_num.clear();
            }
        }
        ms as u64
    }

    pub fn extract_meta_content(&self, html: &str, prop: &str) -> Option<String> {
        let pattern = format!("<meta property=\"{prop}\" content=\"");
        if let Some(i) = html.find(&pattern) {
            let start = i + pattern.len();
            if let Some(end) = html[start..].find('"') {
                return Some(html[start..start + end].to_owned());
            }
        }
        None
    }

    pub fn handle_og_title(&self, html: &str, title: &mut String, artist: &mut String) {
        if let Some(og_title) = self.extract_meta_content(html, "og:title") {
            if let Some(caps) = og_title_regex().captures(&og_title) {
                *title = caps
                    .get(1)
                    .map(|m| m.as_str().to_owned())
                    .unwrap_or_else(|| og_title.clone());
                *artist = caps
                    .get(2)
                    .map(|m| m.as_str().to_owned())
                    .unwrap_or_else(|| "Unknown".to_owned());
            } else {
                *title = og_title;
            }
        }
    }
}
