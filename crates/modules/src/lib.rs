use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use harmony_common::module::{Module, ModuleContext};
use std::sync::Arc;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::Id;
use twilight_util::builder::embed::EmbedBuilder;

use dashmap::DashMap;
use lavende::{LavendeEvent, LavendeManager};
use std::sync::OnceLock;

pub static LAVENDE_MANAGER: OnceLock<Arc<LavendeManager>> = OnceLock::new();
pub static VOICE_STATES: OnceLock<Arc<DashMap<u64, u64>>> = OnceLock::new();
pub static IDLE_TIMERS: OnceLock<Arc<DashMap<String, tokio::sync::watch::Sender<bool>>>> =
    OnceLock::new();

pub struct MusicModule {}

impl MusicModule {
    pub fn new(ctx: Arc<ModuleContext>) -> Self {
        let client_id = harmony_common::ids::get_bot_id().to_string();
        let event_tx = ctx.event_tx.clone();
        let event_tx_clone = event_tx.clone();

        let send_to_shard = move |guild_id_str: String, payload: serde_json::Value| {
            if let Ok(guild_id) = guild_id_str.parse::<u64>() {
                let _ = event_tx.try_send(HarmonyEvent::SendToShard {
                    guild_id: twilight_model::id::Id::new(guild_id),
                    payload: payload.to_string(),
                });
            }
        };

        let manager = LavendeManager::new(client_id, send_to_shard);
        let mut events = manager.subscribe_events();

        let discord = ctx.discord.clone();
        let embed_color = ctx.embed_color;

        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                match event {
                    LavendeEvent::TrackStart { guild_id, track } => {
                        if let Some(timers) = IDLE_TIMERS.get() {
                            if let Some((_, tx)) = timers.remove(&guild_id) {
                                let _ = tx.send(true);
                            }
                        }
                        tracing::info!(
                            "[MUSIC] Track started in {}: {}",
                            guild_id,
                            track.info.title
                        );
                        if let Some(manager) = LAVENDE_MANAGER.get() {
                            if let Some(player) = manager.get_player(&guild_id) {
                                if let Some(channel_val) = player.get_data("text_channel_id") {
                                    if let Some(channel_id_u64) = channel_val.as_u64() {
                                        let channel_id = Id::<ChannelMarker>::new(channel_id_u64);
                                        let embed = EmbedBuilder::new()
                                            .description(format!(
                                                "▶️ **Now Playing:**\n[{}]({}) | `{}`",
                                                track.info.title,
                                                track.info.uri.as_deref().unwrap_or(""),
                                                track.info.author
                                            ))
                                            .color(embed_color)
                                            .build();
                                        let _ = discord
                                            .create_message(channel_id)
                                            .embeds(&[embed])
                                            .await;
                                    }
                                }
                            }
                        }
                    }
                    LavendeEvent::TrackEnd { guild_id, track: _, reason } => {
                        tracing::info!("[MUSIC] Track ended in {}: {:?}", guild_id, reason);
                    }
                    LavendeEvent::QueueEnd { guild_id } => {
                        tracing::info!(
                            "[MUSIC] Queue ended in {}. Starting 60s idle disconnect timer...",
                            guild_id
                        );

                        let (tx, mut rx) = tokio::sync::watch::channel(false);
                        if let Some(timers) = IDLE_TIMERS.get() {
                            timers.insert(guild_id.clone(), tx);
                        }

                        let guild_id_clone = guild_id.clone();
                        let event_tx_clone = event_tx_clone.clone();
                        tokio::spawn(async move {
                            let cancelled = tokio::select! {
                                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => false,
                                _ = rx.changed() => *rx.borrow(),
                            };

                            if let Some(timers) = IDLE_TIMERS.get() {
                                timers.remove(&guild_id_clone);
                            }

                            if !cancelled {
                                tracing::info!("[MUSIC] Player in guild {} has been idle for 60 seconds. Leaving voice channel.", guild_id_clone);
                                if let Ok(guild_id_u64) = guild_id_clone.parse::<u64>() {
                                    let payload = serde_json::json!({
                                        "op": 4,
                                        "d": {
                                            "guild_id": guild_id_clone,
                                            "channel_id": null,
                                            "self_mute": false,
                                            "self_deaf": false
                                        }
                                    })
                                    .to_string();

                                    let tx = event_tx_clone.clone();
                                    let event = HarmonyEvent::SendToShard {
                                        guild_id: Id::new(guild_id_u64),
                                        payload,
                                    };
                                    if let Err(e) = tx.send(event).await {
                                        tracing::error!("[MUSIC] Failed to send voice leave on idle timeout: {}", e);
                                    }
                                }

                                if let Some(manager) = LAVENDE_MANAGER.get() {
                                    if let Some(player) = manager.get_player(&guild_id_clone) {
                                        player.destroy(Some("Idle timeout".to_string())).await;
                                    }
                                }
                            } else {
                                tracing::debug!("[MUSIC] Idle timer for guild {} cancelled (new track started).", guild_id_clone);
                            }
                        });
                    }
                    LavendeEvent::Error { guild_id, message } => {
                        tracing::error!("[MUSIC] Error in {}: {}", guild_id, message);
                    }
                    _ => {}
                }
            }
        });

        let arc_manager = Arc::new(manager);
        let _ = LAVENDE_MANAGER.set(arc_manager.clone());
        let _ = VOICE_STATES.set(Arc::new(DashMap::new()));
        let _ = IDLE_TIMERS.set(Arc::new(DashMap::new()));

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
        _ctx: &ModuleContext,
    ) -> impl std::future::Future<Output = Result<(), HarmonyError>> + Send {
        let lavende_manager = LAVENDE_MANAGER.get().cloned();
        let event_clone = event.clone();

        async move {
            let Some(lavende_manager) = lavende_manager else {
                return Ok(());
            };

            if let HarmonyEvent::Discord(discord_event) = event_clone {
                match discord_event.as_ref() {
                    twilight_model::gateway::event::Event::GuildCreate(gc) => {
                        if let twilight_model::gateway::payload::incoming::GuildCreate::Available(
                            guild,
                        ) = gc.as_ref()
                        {
                            if let Some(states) = VOICE_STATES.get() {
                                for vs in &guild.voice_states {
                                    if let Some(channel_id) = vs.channel_id {
                                        states.insert(vs.user_id.get(), channel_id.get());
                                    }
                                }
                            }
                        }
                    }
                    twilight_model::gateway::event::Event::VoiceStateUpdate(vsu) => {
                        if let Some(states) = VOICE_STATES.get() {
                            if let Some(channel_id) = vsu.channel_id {
                                states.insert(vsu.user_id.get(), channel_id.get());
                            } else {
                                states.remove(&vsu.user_id.get());
                            }
                        }

                        let packet = serde_json::json!({
                            "t": "VOICE_STATE_UPDATE",
                            "d": {
                                "guild_id": vsu.guild_id.map(|id| id.to_string()),
                                "channel_id": vsu.channel_id.map(|id| id.to_string()),
                                "user_id": vsu.user_id.to_string(),
                                "session_id": vsu.session_id,
                                "deaf": vsu.deaf,
                                "mute": vsu.mute,
                                "self_deaf": vsu.self_deaf,
                                "self_mute": vsu.self_mute,
                            }
                        });
                        lavende_manager.send_raw_data(&packet).await;
                    }
                    twilight_model::gateway::event::Event::VoiceServerUpdate(vsu) => {
                        let packet = serde_json::json!({
                            "t": "VOICE_SERVER_UPDATE",
                            "d": {
                                "token": vsu.token,
                                "guild_id": vsu.guild_id.to_string(),
                                "endpoint": vsu.endpoint
                            }
                        });
                        lavende_manager.send_raw_data(&packet).await;
                    }
                    _ => {}
                }
            }

            Ok(())
        }
    }
}
