use crate::dispatcher::EventDispatcher;
use crate::shard_manager::ShardManager;
use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use harmony_messaging::publisher::Publisher;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info};
use twilight_gateway::{EventTypeFlags, StreamExt as _};

const RELEVANT_FLAGS: EventTypeFlags = EventTypeFlags::GUILD_CREATE
    .union(EventTypeFlags::GUILD_DELETE)
    .union(EventTypeFlags::INTERACTION_CREATE)
    .union(EventTypeFlags::MESSAGE_CREATE)
    .union(EventTypeFlags::VOICE_STATE_UPDATE)
    .union(EventTypeFlags::VOICE_SERVER_UPDATE)
    .union(EventTypeFlags::READY)
    .union(EventTypeFlags::RESUMED)
    .union(EventTypeFlags::GATEWAY_HELLO)
    .union(EventTypeFlags::GATEWAY_HEARTBEAT_ACK)
    .union(EventTypeFlags::GATEWAY_INVALIDATE_SESSION)
    .union(EventTypeFlags::GATEWAY_RECONNECT);

use harmony_common::event::SerializableEvent;

fn is_voice_critical(event: &SerializableEvent) -> bool {
    matches!(
        event,
        SerializableEvent::VoiceStateUpdate(_) | SerializableEvent::VoiceServerUpdate(_)
    )
}

use lavende::LavendeManager;

pub struct EventLoop<P: Publisher> {
    shard_manager: ShardManager,
    dispatcher: EventDispatcher<P>,
    rx: tokio::sync::mpsc::Receiver<HarmonyEvent>,
    max_concurrent_tasks: usize,
    lavende_manager: Arc<LavendeManager>,
}

impl<P: Publisher> EventLoop<P> {
    pub fn new(
        shard_manager: ShardManager,
        dispatcher: EventDispatcher<P>,
        rx: tokio::sync::mpsc::Receiver<HarmonyEvent>,
        max_concurrent_tasks: usize,
        lavende_manager: Arc<LavendeManager>,
    ) -> Self {
        Self { shard_manager, dispatcher, rx, max_concurrent_tasks, lavende_manager }
    }
}

impl<P: Publisher + 'static> EventLoop<P> {
    pub async fn run(self) -> Result<(), HarmonyError> {
        info!("Starting gateway event loop");

        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_tasks));
        let mut join_set = tokio::task::JoinSet::new();
        let dispatcher = Arc::new(self.dispatcher);

        let mut senders = std::collections::HashMap::new();
        for shard in &self.shard_manager.shards {
            senders.insert(shard.id().number(), shard.sender());
        }

        let shard_count = self.shard_manager.shards.len() as u64;
        let lavende_manager_shared = self.lavende_manager;

        for mut shard in self.shard_manager.shards {
            let dispatcher = Arc::clone(&dispatcher);
            let semaphore = Arc::clone(&semaphore);
            let lavende_manager = lavende_manager_shared.clone();
            join_set.spawn(async move {
                let shard_id = shard.id();
                info!("Starting event loop for shard {}", shard_id.number());

                while let Some(event_result) = shard.next_event(RELEVANT_FLAGS).await {
                    match event_result {
                        Ok(event) => {
                            if let twilight_model::gateway::event::Event::VoiceServerUpdate(
                                ref vsu,
                            ) = event
                            {
                                let packet = serde_json::json!({
                                    "t": "VOICE_SERVER_UPDATE",
                                    "d": {
                                        "token": vsu.token,
                                        "guild_id": vsu.guild_id.to_string(),
                                        "endpoint": vsu.endpoint
                                    }
                                });
                                let manager = lavende_manager.clone();
                                tokio::spawn(async move {
                                    manager.send_raw_data(&packet).await;
                                });
                            } else if let twilight_model::gateway::event::Event::VoiceStateUpdate(
                                ref vsu,
                            ) = event
                            {
                                let packet = serde_json::json!({
                                    "t": "VOICE_STATE_UPDATE",
                                    "d": {
                                        "guild_id": vsu.0.guild_id.map(|id| id.to_string()),
                                        "channel_id": vsu.0.channel_id.map(|id| id.to_string()),
                                        "user_id": vsu.0.user_id.to_string(),
                                        "session_id": vsu.0.session_id.clone(),
                                        "deaf": vsu.0.deaf,
                                        "mute": vsu.0.mute,
                                        "self_deaf": vsu.0.self_deaf,
                                        "self_mute": vsu.0.self_mute,
                                    }
                                });
                                let manager = lavende_manager.clone();
                                tokio::spawn(async move {
                                    manager.send_raw_data(&packet).await;
                                });
                            }

                            let harmony_event = HarmonyEvent::from(event);

                            if let HarmonyEvent::Discord(ref arc_event) = harmony_event {
                                if is_voice_critical(arc_event) {
                                    let dispatcher = Arc::clone(&dispatcher);
                                    let harmony_event = harmony_event.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = dispatcher.dispatch(harmony_event).await {
                                            error!(
                                                "Failed to dispatch voice event on shard {}: {}",
                                                shard_id.number(),
                                                e
                                            );
                                        }
                                    });
                                    continue;
                                }
                            }

                            let dispatcher = Arc::clone(&dispatcher);
                            let permit = match semaphore.clone().acquire_owned().await {
                                Ok(p) => p,
                                Err(_) => {
                                    error!("Semaphore closed on shard {}", shard_id.number());
                                    break;
                                }
                            };
                            tokio::spawn(async move {
                                let _permit = permit;
                                if let Err(e) = dispatcher.dispatch(harmony_event).await {
                                    error!(
                                        "Failed to dispatch event on shard {}: {}",
                                        shard_id.number(),
                                        e
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            error!("Shard {} error: {}", shard_id.number(), e);
                        }
                    }
                }
                info!("Event loop for shard {} ended", shard_id.number());
            });
        }

        let mut rx = self.rx;
        join_set.spawn(async move {
            while let Some(event) = rx.recv().await {
                if let HarmonyEvent::SendToShard { guild_id, payload } = event {
                    let shard_id = ((guild_id.get() >> 22) % shard_count) as u32;
                    if let Some(sender) = senders.get(&shard_id) {
                        if let Err(e) = sender.send(payload) {
                            error!("Failed to send payload to shard {}: {}", shard_id, e);
                        }
                    } else {
                        error!("Shard {} not found for guild {}", shard_id, guild_id);
                    }
                }
            }
        });

        while let Some(res) = join_set.join_next().await {
            if let Err(e) = res {
                error!("Shard task panicked or failed: {}", e);
            }
        }

        Ok(())
    }
}
