use crate::dispatcher::EventDispatcher;
use crate::shard_manager::ShardManager;
use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use harmony_messaging::publisher::Publisher;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use twilight_gateway::{EventTypeFlags, StreamExt as _};
use twilight_model::gateway::event::Event;

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

fn is_voice_critical(event: &Event) -> bool {
    matches!(event, Event::VoiceStateUpdate(_) | Event::VoiceServerUpdate(_))
}

pub struct EventLoop<P: Publisher> {
    shard_manager: ShardManager,
    dispatcher: EventDispatcher<P>,
    rx: tokio::sync::mpsc::Receiver<HarmonyEvent>,
    max_concurrent_tasks: usize,
}

impl<P: Publisher> EventLoop<P> {
    pub fn new(
        shard_manager: ShardManager,
        dispatcher: EventDispatcher<P>,
        rx: tokio::sync::mpsc::Receiver<HarmonyEvent>,
        max_concurrent_tasks: usize,
    ) -> Self {
        Self { shard_manager, dispatcher, rx, max_concurrent_tasks }
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

        for mut shard in self.shard_manager.shards {
            let dispatcher = Arc::clone(&dispatcher);
            let semaphore = Arc::clone(&semaphore);
            join_set.spawn(async move {
                let shard_id = shard.id();
                info!("Starting event loop for shard {}", shard_id.number());

                while let Some(event_result) = shard.next_event(RELEVANT_FLAGS).await {
                    match event_result {
                        Ok(event) => {
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
                            let permit = semaphore.clone().acquire_owned().await;
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

#[allow(dead_code)]
pub fn handle_lag_warn(lagged_by: u64, consumer: &str) {
    warn!(
        "[{}] Broadcast receiver lagged — {} messages dropped. Increase EVENT_BUS_CAPACITY if this is frequent.",
        consumer, lagged_by
    );
}
