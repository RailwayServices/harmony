use lavende::{LavendePlayer, RepeatMode, Track};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Serialize, Deserialize)]
pub struct SerializableQueue {
    pub tracks: Vec<Track>,
    pub current: Option<Track>,
    pub previous: Vec<Track>,
    pub guild_id: String,
}

#[derive(Serialize, Deserialize)]
pub enum SerializableRepeatMode {
    Off,
    Track,
    Queue,
}

impl From<RepeatMode> for SerializableRepeatMode {
    fn from(mode: RepeatMode) -> Self {
        match mode {
            RepeatMode::Off => Self::Off,
            RepeatMode::Track => Self::Track,
            RepeatMode::Queue => Self::Queue,
        }
    }
}

impl From<SerializableRepeatMode> for RepeatMode {
    fn from(val: SerializableRepeatMode) -> Self {
        match val {
            SerializableRepeatMode::Off => RepeatMode::Off,
            SerializableRepeatMode::Track => RepeatMode::Track,
            SerializableRepeatMode::Queue => RepeatMode::Queue,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PlayerStatePayload {
    pub guild_id: String,
    pub queue: SerializableQueue,
    pub repeat_mode: SerializableRepeatMode,
    pub volume: u32,
    pub paused: bool,
    pub text_channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    pub filters_json: Option<String>,
}

pub async fn sync_player_state(
    guild_id: &str,
    player: &LavendePlayer,
    redis_conn: &mut redis::aio::MultiplexedConnection,
) {
    let queue = player.queue.read().await;
    let repeat_mode = player.repeat_mode.read().await;
    let volume = player.volume.read().await;
    let paused = player.paused.read().await;
    let filters_json = {
        let fm = player.filter_manager.read().await;
        Some(fm.to_json())
    };

    let voice_channel_id = {
        let vs = player.voice_state.read().await;
        vs.voice_channel_id.clone()
    };

    let text_channel_id = player.get_data("text_channel_id").and_then(|v| {
        if let Some(s) = v.as_str() {
            Some(s.to_string())
        } else {
            v.as_u64().map(|n| n.to_string())
        }
    });

    let payload = PlayerStatePayload {
        guild_id: guild_id.to_string(),
        queue: SerializableQueue {
            tracks: queue.tracks.iter().cloned().collect(),
            current: queue.current.clone(),
            previous: queue.previous.clone(),
            guild_id: queue.guild_id.clone(),
        },
        repeat_mode: SerializableRepeatMode::from(repeat_mode.clone()),
        volume: *volume,
        paused: *paused,
        text_channel_id,
        voice_channel_id,
        filters_json,
    };

    match serde_json::to_string(&payload) {
        Ok(json) => {
            let key = format!("harmony:player_state:{}", guild_id);
            if let Err(e) = redis_conn.set::<_, _, ()>(&key, json).await {
                tracing::error!(
                    "[STATE_SYNC] Failed to save player state to Redis for {}: {}",
                    guild_id,
                    e
                );
            }
        }
        Err(e) => {
            tracing::error!("[STATE_SYNC] Failed to serialize player state for {}: {}", guild_id, e)
        }
    }
}

pub async fn restore_player_state(
    _guild_id: &str,
    player: &LavendePlayer,
    payload: PlayerStatePayload,
) {
    {
        let mut q = player.queue.write().await;
        q.tracks = VecDeque::from(payload.queue.tracks);
        q.current = payload.queue.current;
        q.previous = payload.queue.previous;
        q.guild_id = payload.queue.guild_id;
    }
    {
        let mut mode = player.repeat_mode.write().await;
        *mode = payload.repeat_mode.into();
    }
    {
        let mut vol = player.volume.write().await;
        *vol = payload.volume;
    }
    {
        let mut p = player.paused.write().await;
        *p = payload.paused;
    }
    if let Some(ch) = payload.text_channel_id {
        player.set_data("text_channel_id", serde_json::Value::String(ch));
    }
    if let Some(json_str) = payload.filters_json {
        player.set_filters(json_str).await;
    }
}

pub async fn delete_player_state(
    guild_id: &str,
    redis_conn: &mut redis::aio::MultiplexedConnection,
) {
    let key = format!("harmony:player_state:{}", guild_id);
    if let Err(e) = redis_conn.del::<_, ()>(&key).await {
        tracing::error!(
            "[STATE_SYNC] Failed to delete player state from Redis for {}: {}",
            guild_id,
            e
        );
    }
}
