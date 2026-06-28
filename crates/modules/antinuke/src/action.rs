use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_common::module::ModuleContext;
use tracing::{error, info};
use twilight_model::gateway::event::Event;

pub struct ActionExecutor {}

impl ActionExecutor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute_penalty(
        &self,
        event: &RailwayEvent,
        ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        if let RailwayEvent::Discord(discord_event) = event {
            if let Event::BanAdd(ban) = &**discord_event {
                let guild_id = ban.guild_id;
                let malicious_user_id = ban.user.id;

                info!(
                    "Executing antinuke penalty: Banning user {} from guild {}",
                    malicious_user_id, guild_id
                );

                if let Err(e) = ctx.discord.create_ban(guild_id, malicious_user_id).await {
                    error!("Failed to ban user {}: {}", malicious_user_id, e);
                }
            }
        }
        Ok(())
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}
