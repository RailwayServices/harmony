use crate::dispatcher::EventDispatcher;
use crate::shard_manager::ShardManager;
use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_messaging::publisher::Publisher;
use tracing::{error, info};
use twilight_gateway::{EventTypeFlags, StreamExt};

pub struct EventLoop<P: Publisher> {
    shard_manager: ShardManager,
    dispatcher: EventDispatcher<P>,
}

impl<P: Publisher> EventLoop<P> {
    pub fn new(shard_manager: ShardManager, dispatcher: EventDispatcher<P>) -> Self {
        Self { shard_manager, dispatcher }
    }

    pub async fn run(mut self) -> Result<(), RailwayError> {
        info!("Starting gateway event loop");

        loop {
            let event_result = self.shard_manager.shard.next_event(EventTypeFlags::all()).await;

            match event_result {
                Some(Ok(event)) => {
                    let railway_event = RailwayEvent::from(event);
                    if let Err(e) = self.dispatcher.dispatch(railway_event).await {
                        error!("Failed to dispatch event: {}", e);
                    }
                }
                Some(Err(e)) => {
                    error!("Shard error: {}", e);
                    return Err(RailwayError::Internal(format!("Shard error: {}", e)));
                }
                None => {
                    info!("Shard event stream ended");
                    break;
                }
            }
        }
        Ok(())
    }
}
