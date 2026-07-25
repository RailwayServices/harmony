use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track};
use crate::sources::deezer::{DeezerSource, PRIVATE_API_BASE, REC_ARTIST_PREFIX, REC_TRACK_PREFIX};
use serde_json::Value;

pub async fn get_recommendations(source: &DeezerSource, query: &str) -> LoadResult {
    let tokens = match source.token_tracker.get_token().await {
        Some(t) => t,
        None => return LoadResult::Empty {},
    };

    let (method, payload) = if let Some(artist_id) = query.strip_prefix(REC_ARTIST_PREFIX) {
        (
            "song.getSmartRadio",
            serde_json::json!({ "art_id": artist_id }),
        )
    } else {
        let track_id = query.strip_prefix(REC_TRACK_PREFIX).unwrap_or(query);
        (
            "song.getSearchTrackMix",
            serde_json::json!({ "sng_id": track_id, "start_with_input_track": "true" }),
        )
    };

    let url = format!(
        "{PRIVATE_API_BASE}?method={method}&input=3&api_version=1.0&api_token={}",
        tokens.api_token
    );

    let res = match source
        .client
        .post(&url)
        .header(
            "Cookie",
            format!(
                "sid={}; dzr_uniq_id={}",
                tokens.session_id, tokens.dzr_uniq_id
            ),
        )
        .json(&payload)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return LoadResult::Empty {},
    };

    let json: Value = res.json().await.unwrap_or(Value::Null);
    let data = json.get("results").and_then(|r| r.get("data"));

    let tracks: Vec<Track> = if let Some(arr) = data.and_then(|d| d.as_array()) {
        arr.iter()
            .filter_map(|item| source.parse_recommendation_track(item))
            .collect()
    } else if let Some(obj) = data.and_then(|d| d.as_object()) {
        obj.values()
            .filter_map(|item| source.parse_recommendation_track(item))
            .collect()
    } else {
        Vec::new()
    };

    if tracks.is_empty() {
        return LoadResult::Empty {};
    }

    LoadResult::Playlist(PlaylistData {
        info: PlaylistInfo {
            name: "Deezer Recommendations".to_owned(),
            selected_track: -1,
        },
        plugin_info: serde_json::json!({
            "type": "recommendations",
            "totalTracks": tracks.len()
        }),
        tracks,
    })
}
