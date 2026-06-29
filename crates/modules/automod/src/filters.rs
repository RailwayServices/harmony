use chrono::{Duration, Utc};
use once_cell::sync::Lazy;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::models::automod_rule::{ActionType, TriggerType};
use railway_database::repository::automod_repository::AutomodRepository;
use regex::Regex;
use tracing::info;
use twilight_model::gateway::payload::incoming::{MessageCreate, MessageUpdate};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_model::util::Timestamp;

static INVITE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(discord\.gg/|discord\.com/invite/|discordapp\.com/invite/)").unwrap()
});

pub async fn process_message(ctx: &ModuleContext, msg: &MessageCreate) -> Result<(), RailwayError> {
    if msg.author.bot {
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let rules = AutomodRepository::get_rules(&ctx.db, guild_id.get() as i64).await?;
    if rules.is_empty() {
        return Ok(());
    }

    let mut is_deleted = false;

    for rule in rules {
        if let Ok(trigger) = TriggerType::try_from(rule.trigger_type) {
            match trigger {
                TriggerType::AntiLink => {
                    if !is_deleted && INVITE_REGEX.is_match(&msg.content) {
                        info!(
                            "[AUTOMOD] AntiLink triggered for user {} in guild {}",
                            msg.author.id, guild_id
                        );
                        apply_action(ctx, guild_id, msg, rule.action_type).await?;
                        is_deleted = true; // Avoid multiple deletes
                    }
                }
                TriggerType::Spam => {
                    if check_spam(ctx, guild_id, msg.author.id.get()).await? {
                        info!(
                            "[AUTOMOD] Spam triggered for user {} in guild {}",
                            msg.author.id, guild_id
                        );
                        apply_action(ctx, guild_id, msg, rule.action_type).await?;
                    }
                }
                TriggerType::GhostPing => {
                    // Handled separately in ghost_ping module
                }
            }
        }
    }

    Ok(())
}

async fn check_spam(
    ctx: &ModuleContext,
    guild_id: Id<GuildMarker>,
    user_id: u64,
) -> Result<bool, RailwayError> {
    let mut cache = ctx.cache.clone();
    let key = format!("automod:spam:{}:{}", guild_id.get(), user_id);

    let count: i64 =
        redis::cmd("INCR").arg(&key).query_async(&mut cache).await.map_err(RailwayError::Cache)?;

    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(5)
            .query_async(&mut cache)
            .await
            .map_err(RailwayError::Cache)?;
    }

    if count > 5 {
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn apply_action(
    ctx: &ModuleContext,
    guild_id: Id<GuildMarker>,
    msg: &MessageCreate,
    action_raw: i16,
) -> Result<(), RailwayError> {
    if let Ok(action) = ActionType::try_from(action_raw) {
        match action {
            ActionType::DeleteMessage => {
                let _ = ctx.discord.delete_message(msg.channel_id, msg.id).await;
            }
            ActionType::Timeout => {
                let timeout_until = Utc::now() + Duration::minutes(5);
                let timestamp = Timestamp::from_secs(timeout_until.timestamp()).unwrap();
                let _ = ctx
                    .discord
                    .update_guild_member(guild_id, msg.author.id)
                    .communication_disabled_until(Some(timestamp))
                    .await;
            }
            ActionType::DeleteAndTimeout => {
                let _ = ctx.discord.delete_message(msg.channel_id, msg.id).await;

                let timeout_until = Utc::now() + Duration::minutes(5);
                let timestamp = Timestamp::from_secs(timeout_until.timestamp()).unwrap();
                let _ = ctx
                    .discord
                    .update_guild_member(guild_id, msg.author.id)
                    .communication_disabled_until(Some(timestamp))
                    .await;
            }
        }
    }
    Ok(())
}

pub async fn process_message_update(
    _ctx: &ModuleContext,
    _msg: &MessageUpdate,
) -> Result<(), RailwayError> {
    Ok(())
}
