use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::repository::automod_repository::AutomodRepository;
use std::sync::Arc;
use twilight_model::gateway::payload::incoming::{MessageCreate, MessageUpdate};

pub async fn process_message(
    ctx: Arc<ModuleContext>,
    msg: &MessageCreate,
) -> Result<(), RailwayError> {
    if msg.author.bot {
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let rules = AutomodRepository::get_rules(&ctx.db, guild_id.get() as i64).await?;

    // In a real implementation, we'd iterate over rules and check regex/filters
    if rules.is_empty() {
        return Ok(());
    }

    Ok(())
}

pub async fn process_message_update(
    _ctx: Arc<ModuleContext>,
    _msg: &MessageUpdate,
) -> Result<(), RailwayError> {
    // Similar to process_message
    Ok(())
}
