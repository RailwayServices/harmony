use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo};
use serde_json::{Value, json};

impl super::YandexMusicSource {
    pub fn build_playlist_from_search(&self, item: &Value, r#type: &str) -> Option<PlaylistData> {
        if !item["available"].as_bool().unwrap_or(false) {
            return None;
        }
        let name = match r#type {
            "artist" => item["name"].as_str()?.to_string(),
            _ => item["title"].as_str()?.to_string(),
        };
        Some(PlaylistData {
            info: PlaylistInfo {
                name,
                selected_track: -1,
            },
            plugin_info: json!({ "type": r#type }),
            tracks: Vec::new(),
        })
    }

    pub fn build_playlist_result(&self, data: Value) -> LoadResult {
        let tracks = self.parse_tracks(&data["tracks"]);
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        let title = if data["kind"].as_u64() == Some(3) {
            let owner = data["owner"]["name"]
                .as_str()
                .or(data["owner"]["login"].as_str())
                .unwrap_or("User");
            format!("{}'s liked songs", owner)
        } else {
            data["title"]
                .as_str()
                .unwrap_or("Yandex Music Playlist")
                .to_string()
        };
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: title,
                selected_track: -1,
            },
            plugin_info: json!({ "type": "playlist" }),
            tracks,
        })
    }

    pub fn parse_tracks(&self, data: &Value) -> Vec<Track> {
        let mut tracks = Vec::new();
        if let Some(arr) = data.as_array() {
            for item in arr {
                let track_json = if item.get("track").is_some() {
                    &item["track"]
                } else {
                    item
                };
                if let Some(track) = self.build_track(track_json) {
                    tracks.push(track);
                }
            }
        }
        tracks
    }

    pub fn build_track(&self, data: &Value) -> Option<Track> {
        if !data["available"].as_bool().unwrap_or(false) {
            return None;
        }
        let id = data["id"]
            .as_u64()
            .map(|n| n.to_string())
            .or(data["id"].as_str().map(|s| s.to_string()))?;
        let title = data["title"].as_str()?;
        let author = self.parse_artist(data);
        let duration = data["durationMs"].as_u64().unwrap_or(0);
        let uri = Some(format!("https://music.yandex.ru/track/{}", id));
        let artwork_url = self.parse_cover_uri(data);
        Some(Track::new(TrackInfo {
            identifier: id,
            is_seekable: true,
            author,
            length: duration,
            is_stream: false,
            position: 0,
            title: title.to_string(),
            uri,
            artwork_url,
            isrc: data["isrc"].as_str().map(|s| s.to_string()),
            source_name: "yandexmusic".to_string(),
        }))
    }

    fn parse_artist(&self, data: &Value) -> String {
        if let Some(arr) = data["artists"].as_array() {
            return arr
                .iter()
                .filter_map(|a| a["name"].as_str())
                .collect::<Vec<_>>()
                .join(", ");
        }
        "Unknown Artist".to_string()
    }

    fn parse_cover_uri(&self, data: &Value) -> Option<String> {
        let uri = data["ogImage"]
            .as_str()
            .or(data["coverUri"].as_str())
            .or(data["cover"]["uri"].as_str())?;
        Some(format!("https://{}", uri.replace("%%", "400x400")))
    }
}
