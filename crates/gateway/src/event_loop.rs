use crate::dispatcher::EventDispatcher;
use crate::shard_manager::ShardManager;
use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_messaging::publisher::Publisher;
use std::sync::Arc;
use tracing::{error, info};
use twilight_gateway::StreamExt as _;

pub struct EventLoop<P: Publisher> {
    shard_manager: ShardManager,
    dispatcher: EventDispatcher<P>,
}

impl<P: Publisher> EventLoop<P> {
    pub fn new(shard_manager: ShardManager, dispatcher: EventDispatcher<P>) -> Self {
        Self { shard_manager, dispatcher }
    }
}

impl<P: Publisher + 'static> EventLoop<P> {
    pub async fn run(self) -> Result<(), RailwayError> {
        info!("Starting gateway event loop");

        let mut join_set = tokio::task::JoinSet::new();
        let dispatcher = Arc::new(self.dispatcher);

        for mut shard in self.shard_manager.shards {
            let dispatcher = Arc::clone(&dispatcher);
            join_set.spawn(async move {
                let shard_id = shard.id();
                info!("Starting event loop for shard {}", shard_id.number());

                while let Some(event_result) =
                    shard.next_event(twilight_gateway::EventTypeFlags::all()).await
                {
                    match event_result {
                        Ok(event) => {
                            let railway_event = RailwayEvent::from(event);
                            if let Err(e) = dispatcher.dispatch(railway_event).await {
                                error!(
                                    "Failed to dispatch event on shard {}: {}",
                                    shard_id.number(),
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            error!("Shard {} error: {}", shard_id.number(), e);
                        }
                    }
                }
                info!("Event loop for shard {} ended", shard_id.number());
            });
        }

        while let Some(res) = join_set.join_next().await {
            if let Err(e) = res {
                error!("Shard task panicked or failed: {}", e);
            }
        }

        Ok(())
    }
}
