use chrono::{Duration, Utc};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::models::automod_rule::{ActionType, TriggerType};
use railway_database::repository::automod_repository::AutomodRepository;
use tracing::info;
use twilight_model::gateway::payload::incoming::{MessageCreate, MessageDelete};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
    Id,
};
use twilight_model::util::Timestamp;

#[derive(Clone)]
pub struct CachedMessage {
    pub author_id: Id<UserMarker>,
    pub content: String,
    pub mentions: Vec<Id<UserMarker>>,
}

static MESSAGE_CACHE: Lazy<DashMap<Id<MessageMarker>, CachedMessage>> = Lazy::new(DashMap::new);

pub async fn cache_message(msg: &MessageCreate) {
    if msg.author.bot {
        return;
    }

    if msg.mentions.is_empty() {
        return; // Only cache messages with mentions to save memory
    }

    let mentions = msg.mentions.iter().map(|m| m.id).collect();

    MESSAGE_CACHE.insert(
        msg.id,
        CachedMessage { author_id: msg.author.id, content: msg.content.clone(), mentions },
    );

    // To prevent infinite growth, we should technically sweep the cache
    // but DashMap is fast enough for small bots. A proper TTL cache like `moka` is better for production.
}

pub async fn handle_message_delete(
    ctx: &ModuleContext,
    msg: &MessageDelete,
) -> Result<(), RailwayError> {
    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    if let Some((_, cached)) = MESSAGE_CACHE.remove(&msg.id) {
        let rules = AutomodRepository::get_rules(&ctx.db, guild_id.get() as i64).await?;

        for rule in rules {
            if let Ok(TriggerType::GhostPing) = TriggerType::try_from(rule.trigger_type) {
                info!(
                    "[AUTOMOD] Ghost ping detected by user {} in guild {}",
                    cached.author_id, guild_id
                );

                // Alert the channel
                let content = format!("<@{}> Ghost pinging is not allowed! You mentioned {} users and deleted the message.", cached.author_id, cached.mentions.len());
                let _ = ctx.discord.create_message(msg.channel_id).content(&content).await;

                apply_action(ctx, guild_id, msg.channel_id, cached.author_id, rule.action_type)
                    .await?;
                break;
            }
        }
    }

    Ok(())
}

async fn apply_action(
    ctx: &ModuleContext,
    guild_id: Id<GuildMarker>,
    _channel_id: Id<ChannelMarker>, // Cannot delete a message that is already deleted
    author_id: Id<UserMarker>,
    action_raw: i16,
) -> Result<(), RailwayError> {
    if let Ok(ActionType::Timeout | ActionType::DeleteAndTimeout) = ActionType::try_from(action_raw)
    {
        let timeout_until = Utc::now() + Duration::minutes(5);
        let timestamp = Timestamp::from_secs(timeout_until.timestamp()).unwrap();
        let _ = ctx
            .discord
            .update_guild_member(guild_id, author_id)
            .communication_disabled_until(Some(timestamp))
            .await;
    }
    Ok(())
}
