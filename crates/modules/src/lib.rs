use dashmap::DashMap;
use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use harmony_common::module::{Module, ModuleContext};
use harmony_common::music_ipc::MusicResponse;
use std::sync::Arc;
use std::sync::OnceLock;

pub static MUSIC_RESPONSES: OnceLock<
    Arc<DashMap<String, tokio::sync::oneshot::Sender<MusicResponse>>>,
> = OnceLock::new();
pub static IDLE_TIMERS: OnceLock<Arc<DashMap<String, tokio::sync::watch::Sender<bool>>>> =
    OnceLock::new();

pub struct MusicModule {}

impl MusicModule {
    pub fn new(_ctx: Arc<ModuleContext>, redis_url: String) -> Self {
        MUSIC_RESPONSES.get_or_init(|| Arc::new(DashMap::new()));
        let _ = IDLE_TIMERS.set(Arc::new(DashMap::new()));

        tokio::spawn(async move {
            let redis_client = match redis::Client::open(redis_url) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to open redis client for MusicModule responses: {}", e);
                    return;
                }
            };

            let mut pubsub_conn = match redis_client.get_async_pubsub().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to get pubsub conn for MusicModule responses: {}", e);
                    return;
                }
            };

            if let Err(e) = pubsub_conn.psubscribe("harmony:music:responses:*").await {
                tracing::error!("Failed to psubscribe to music responses: {}", e);
                return;
            }

            tracing::info!("[MUSIC] Started listening for IPC responses from Gateway.");

            use futures::StreamExt;
            let mut stream = pubsub_conn.into_on_message();
            while let Some(msg) = stream.next().await {
                let channel = msg.get_channel_name();
                let req_id = match channel.split(':').next_back() {
                    Some(id) => id,
                    None => continue,
                };

                let payload: String = match msg.get_payload() {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                let response: MusicResponse = match serde_json::from_str(&payload) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("Failed to deserialize music response: {}", e);
                        continue;
                    }
                };

                if let Some(map) = MUSIC_RESPONSES.get() {
                    if let Some((_, sender)) = map.remove(req_id) {
                        let _ = sender.send(response);
                    }
                }
            }
        });

        Self {}
    }
}

impl Module for MusicModule {
    fn name(&self) -> &'static str {
        "Music Module"
    }

    fn handle_event(
        &self,
        event: &HarmonyEvent,
        ctx: &ModuleContext,
    ) -> impl std::future::Future<Output = Result<(), HarmonyError>> + Send {
        let event_clone = event.clone();
        let mut redis_conn = ctx.cache.clone();

        async move {
            use redis::AsyncCommands;

            if let HarmonyEvent::Discord(discord_event) = event_clone {
                match discord_event.as_ref() {
                    harmony_common::event::SerializableEvent::GuildCreate(gc) => {
                        if let twilight_model::gateway::payload::incoming::GuildCreate::Available(
                            guild,
                        ) = gc.as_ref()
                        {
                            for vs in &guild.voice_states {
                                if let Some(channel_id) = vs.channel_id {
                                    let _: Result<(), _> = redis_conn
                                        .hset(
                                            "harmony:voice_states",
                                            vs.user_id.to_string(),
                                            channel_id.to_string(),
                                        )
                                        .await;
                                }
                            }
                        }
                    }
                    harmony_common::event::SerializableEvent::VoiceStateUpdate(vsu) => {
                        if let Some(channel_id) = vsu.channel_id {
                            let _: Result<(), _> = redis_conn
                                .hset(
                                    "harmony:voice_states",
                                    vsu.user_id.to_string(),
                                    channel_id.to_string(),
                                )
                                .await;
                        } else {
                            let _: Result<(), _> = redis_conn
                                .hdel("harmony:voice_states", vsu.user_id.to_string())
                                .await;
                        }
                    }
                    _ => {}
                }
            }

            Ok(())
        }
    }
}
