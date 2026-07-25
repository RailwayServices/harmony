use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo};
use crate::sources::deezer::DeezerSource;

pub async fn get_track(source: &DeezerSource, id: &str) -> LoadResult {
    match source.get_json_public(&format!("track/{id}")).await {
        Some(json) => match source.parse_track(&json) {
            Some(track) => LoadResult::Track(track),
            None => LoadResult::Empty {},
        },
        None => LoadResult::Empty {},
    }
}

pub async fn get_album(source: &DeezerSource, id: &str) -> LoadResult {
    let json = match source.get_json_public(&format!("album/{id}")).await {
        Some(j) => j,
        None => return LoadResult::Empty {},
    };

    let tracks_json = match source
        .get_json_public(&format!("album/{id}/tracks?limit=10000"))
        .await
    {
        Some(j) => j,
        None => return LoadResult::Empty {},
    };

    let mut tracks = Vec::new();
    let artwork_url = json
        .get("cover_xl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());

    if let Some(data) = tracks_json.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(mut track) = source.parse_track(item) {
                if track.info.artwork_url.is_none() {
                    track.info.artwork_url = artwork_url.clone();
                }
                tracks.push(track);
            }
        }
    }

    if tracks.is_empty() {
        return LoadResult::Empty {};
    }

    LoadResult::Playlist(PlaylistData {
        info: PlaylistInfo {
            name: json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Album")
                .to_owned(),
            selected_track: -1,
        },
        plugin_info: serde_json::json!({
            "type": "album",
            "url": format!("https://www.deezer.com/album/{id}"),
            "artworkUrl": json.get("cover_xl").and_then(|v| v.as_str()),
            "author": json.get("artist").and_then(|v| v.get("name")).and_then(|v| v.as_str()),
            "totalTracks": json.get("nb_tracks").and_then(|v| v.as_u64()).unwrap_or(tracks.len() as u64)
        }),
        tracks,
    })
}

pub async fn get_playlist(source: &DeezerSource, id: &str) -> LoadResult {
    let json = match source.get_json_public(&format!("playlist/{id}")).await {
        Some(j) => j,
        None => return LoadResult::Empty {},
    };

    let tracks_json = match source
        .get_json_public(&format!("playlist/{id}/tracks?limit=10000"))
        .await
    {
        Some(j) => j,
        None => return LoadResult::Empty {},
    };

    let mut tracks = Vec::new();
    if let Some(data) = tracks_json.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(track) = source.parse_track(item) {
                tracks.push(track);
            }
        }
    }

    if tracks.is_empty() {
        return LoadResult::Empty {};
    }

    LoadResult::Playlist(PlaylistData {
        info: PlaylistInfo {
            name: json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Playlist")
                .to_owned(),
            selected_track: -1,
        },
        plugin_info: serde_json::json!({
            "type": "playlist",
            "url": format!("https://www.deezer.com/playlist/{id}"),
            "artworkUrl": json.get("picture_xl").and_then(|v| v.as_str()),
            "author": json.get("creator").and_then(|v| v.get("name")).and_then(|v| v.as_str()),
            "totalTracks": json.get("nb_tracks").and_then(|v| v.as_u64()).unwrap_or(tracks.len() as u64)
        }),
        tracks,
    })
}

pub async fn get_artist(source: &DeezerSource, id: &str) -> LoadResult {
    let json = match source.get_json_public(&format!("artist/{id}")).await {
        Some(j) => j,
        None => return LoadResult::Empty {},
    };

    let tracks_json = match source
        .get_json_public(&format!("artist/{id}/top?limit=50"))
        .await
    {
        Some(j) => j,
        None => return LoadResult::Empty {},
    };

    let artwork_url = json
        .get("picture_xl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());

    let author = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Artist")
        .to_owned();

    let mut tracks = Vec::new();
    if let Some(data) = tracks_json.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(mut track) = source.parse_track(item) {
                if track.info.artwork_url.is_none() {
                    track.info.artwork_url = artwork_url.clone();
                }
                tracks.push(track);
            }
        }
    }

    if tracks.is_empty() {
        return LoadResult::Empty {};
    }

    LoadResult::Playlist(PlaylistData {
        info: PlaylistInfo {
            name: format!("{author}'s Top Tracks"),
            selected_track: -1,
        },
        plugin_info: serde_json::json!({
            "type": "artist",
            "url": format!("https://www.deezer.com/artist/{id}"),
            "artworkUrl": json.get("picture_xl").and_then(|v| v.as_str()),
            "author": author,
            "totalTracks": tracks.len()
        }),
        tracks,
    })
}
