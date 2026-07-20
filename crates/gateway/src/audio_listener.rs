use futures::StreamExt;
use harmony_common::music_ipc::{MusicCommand, MusicResponse};
use lavende::LavendeManager;
use redis::AsyncCommands;
use std::sync::Arc;
use tracing::{error, info};

pub struct AudioListener {
    manager: Arc<LavendeManager>,
    redis_url: String,
}

impl AudioListener {
    pub fn new(manager: Arc<LavendeManager>, redis_url: String) -> Self {
        Self { manager, redis_url }
    }

    pub async fn run(self) {
        let cache_client = match harmony_cache::client::CacheClient::connect(&self.redis_url).await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to connect cache client for AudioListener: {}", e);
                return;
            }
        };
        
        let redis_client = cache_client.client;

        let mut pubsub_conn = match redis_client.get_async_pubsub().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get pubsub conn for AudioListener: {}", e);
                return;
            }
        };

        if let Err(e) = pubsub_conn.subscribe("harmony:music:requests").await {
            error!("Failed to subscribe to harmony:music:requests: {}", e);
            return;
        }

        let publish_client = redis_client.clone();

        info!("AudioListener started, waiting for music commands...");

        let mut stream = pubsub_conn.into_on_message();
        while let Some(msg) = stream.next().await {
            let payload: String = match msg.get_payload() {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to get payload from music request: {}", e);
                    continue;
                }
            };

            let command: MusicCommand = match serde_json::from_str(&payload) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to deserialize music command: {}", e);
                    continue;
                }
            };

            let manager = self.manager.clone();
            let pub_client = publish_client.clone();

            tokio::spawn(async move {
                Self::handle_command(manager, pub_client, command).await;
            });
        }
    }

    async fn handle_command(
        manager: Arc<LavendeManager>,
        pub_client: redis::Client,
        command: MusicCommand,
    ) {
        let mut publish_conn = match pub_client.get_multiplexed_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!("AudioListener failed to get publish connection: {}", e);
                return;
            }
        };

        match command {
            MusicCommand::Play { req_id, guild_id, channel_id, text_channel_id, query } => {
                let player = manager.get_or_create_player(&guild_id);

                let ((), search_result) = tokio::join!(
                    player.connect(Some(channel_id), false, true),
                    player.search(&query)
                );

                let response = match search_result {
                    Ok(result) => {
                        let should_play = {
                            let mut q = player.queue.write().await;
                            let was_empty = q.current.is_none() && q.tracks.is_empty();
                            match &result {
                                lavende::LoadResult::Track(track) => {
                                    q.add(track.clone());
                                }
                                lavende::LoadResult::Search(tracks) => {
                                    if let Some(track) = tracks.first() {
                                        q.add(track.clone());
                                    }
                                }
                                lavende::LoadResult::Playlist(playlist) => {
                                    q.add_multiple(playlist.tracks.clone());
                                }
                                _ => {}
                            }
                            was_empty
                        };

                        if should_play {
                            if let Some(ch) = text_channel_id
                                && let Ok(parsed_ch) = ch.parse::<u64>()
                            {
                                player.set_data("text_channel_id", serde_json::json!(parsed_ch));
                            }
                            let _ = player.play().await;
                        }

                        MusicResponse::PlayResult { req_id: req_id.clone(), result }
                    }
                    Err(e) => MusicResponse::Error { req_id: req_id.clone(), message: e },
                };

                let res_payload = match serde_json::to_string(&response) {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Failed to serialize play response: {}", e);
                        return;
                    }
                };
                let channel = format!("harmony:music:responses:{}", req_id);
                let _: Result<(), _> = publish_conn.publish(channel, res_payload).await;
            }
            MusicCommand::Pause { guild_id } => {
                if let Some(player) = manager.get_player(&guild_id) {
                    let _ = player.pause(true).await;
                }
            }
            MusicCommand::Stop { guild_id } => {
                if let Some(player) = manager.get_player(&guild_id) {
                    let _ = player.stop().await;
                }
            }
            MusicCommand::Resume { guild_id } => {
                if let Some(player) = manager.get_player(&guild_id) {
                    let _ = player.pause(false).await;
                }
            }
            MusicCommand::Skip { guild_id } => {
                if let Some(player) = manager.get_player(&guild_id) {
                    let _ = player.skip().await;
                }
            }
            MusicCommand::Queue { req_id, guild_id } => {
                if let Some(player) = manager.get_player(&guild_id) {
                    let q = player.queue.read().await;
                    let tracks: Vec<_> = q.tracks.clone().into();
                    let current = q.current.clone();
                    let response =
                        MusicResponse::QueueResult { req_id: req_id.clone(), tracks, current };
                    let res_payload = match serde_json::to_string(&response) {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Failed to serialize queue response: {}", e);
                            return;
                        }
                    };
                    let channel = format!("harmony:music:responses:{}", req_id);
                    let _: Result<(), _> = publish_conn.publish(channel, res_payload).await;
                }
            }
            MusicCommand::Filter { guild_id, filter_type } => {
                if let Some(player) = manager.get_player(&guild_id) {
                    let filter_json = match filter_type.as_str() {
                        "bassboost" => {
                            r#"{"equalizer":[{"band":0,"gain":0.2},{"band":1,"gain":0.15},{"band":2,"gain":0.1}]}"#
                        }
                        "nightcore" => r#"{"timescale":{"speed":1.3,"pitch":1.3,"rate":1.0}}"#,
                        "vaporwave" => r#"{"timescale":{"speed":0.8,"pitch":0.8,"rate":1.0}}"#,
                        "8d" => r#"{"rotation":{"rotationHz":0.2}}"#,
                        "tremolo" => r#"{"tremolo":{"frequency":2.0,"depth":0.5}}"#,
                        "vibrato" => r#"{"vibrato":{"frequency":2.0,"depth":0.5}}"#,
                        "clear" => r#"{}"#,
                        unknown => {
                            error!("Unknown filter type: {}", unknown);
                            return;
                        }
                    };

                    player.set_filters(filter_json.to_string()).await;
                }
            }
        }
    }
}
