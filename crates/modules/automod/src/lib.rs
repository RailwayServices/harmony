pub mod actions;
pub mod filters;
pub mod ghost_ping;

use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_common::module::{Module, ModuleContext};

pub struct AutomodModule {}

impl AutomodModule {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AutomodModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutomodModule {
    fn name(&self) -> &'static str {
        "automod"
    }

    async fn handle_event(
        &self,
        event: &railway_common::event::RailwayEvent,
        ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        use twilight_model::gateway::event::Event;

        if let RailwayEvent::Discord(twilight_event) = event {
            match twilight_event.as_ref() {
                Event::MessageCreate(msg) => {
                    let msg_create = msg.as_ref();
                    // 1. Check Anti-Link and Spam
                    filters::process_message(ctx, msg_create).await?;
                    // 2. Cache for Ghost Ping detection
                    ghost_ping::cache_message(msg_create).await;
                }
                Event::MessageDelete(msg) => {
                    // Check Ghost Ping
                    ghost_ping::handle_message_delete(ctx, msg).await?;
                }
                Event::MessageUpdate(msg) => {
                    filters::process_message_update(ctx, msg.as_ref()).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
