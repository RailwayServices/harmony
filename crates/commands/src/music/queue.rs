use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixContext;
use crate::core::traits::{AppCommand, PrefixCommand};
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_common::music_ipc::{MusicCommand, MusicResponse};
use harmony_modules::MUSIC_RESPONSES;
use tokio::time::{Duration, timeout};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::embed::EmbedBuilder;

#[derive(CommandModel, CreateCommand)]
#[command(name = "queue", desc = "View the current queue")]
pub struct QueueCommand {}

pub struct QueueAppCommand;
#[async_trait::async_trait]
impl AppCommand for QueueAppCommand {
    fn name(&self) -> &'static str {
        "queue"
    }
    fn register(&self) -> twilight_model::application::command::Command {
        QueueCommand::create_command().into()
    }
    async fn handle(
        &self,
        ctx: &InteractionContext,
        _data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_queue(ctx, module_ctx).await
    }
}

pub struct QueuePrefixCommand;
#[async_trait::async_trait]
impl PrefixCommand for QueuePrefixCommand {
    fn aliases(&self) -> Vec<&'static str> {
        vec!["queue", "q"]
    }
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_prefix_queue(ctx, module_ctx).await
    }
}

fn format_duration(ms: u64) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let seconds = seconds % 60;

    if minutes >= 60 {
        let hours = minutes / 60;
        let minutes = minutes % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

pub async fn handle_queue(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    interaction_ctx.defer(module_ctx).await?;

    let req_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();

    if let Some(map) = MUSIC_RESPONSES.get() {
        map.insert(req_id.clone(), tx);
    } else {
        return Err(HarmonyError::Internal("Music responses map not initialized".to_string()));
    }

    let cmd = MusicCommand::Queue { req_id: req_id.clone(), guild_id: guild_id.to_string() };

    let payload = serde_json::to_string(&cmd).unwrap_or_default();

    {
        use redis::AsyncCommands;
        let mut redis_conn = module_ctx.cache.clone();
        let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;
    }

    let response = match timeout(Duration::from_secs(5), rx).await {
        Ok(Ok(res)) => res,
        _ => {
            if let Some(map) = MUSIC_RESPONSES.get() {
                map.remove(&req_id);
            }
            let embed = EmbedBuilder::new()
                .description("❌ Timeout waiting for queue data.")
                .color(0xFF0000)
                .build();
            let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            return Ok(());
        }
    };

    if let MusicResponse::QueueResult { tracks, current, .. } = response {
        let mut description = String::new();
        let mut total_duration = 0;

        if let Some(current) = &current {
            description.push_str(&format!(
                "**🎵 Now Playing:**\n[{}]({}) | `{}`\n\n",
                current.info.title,
                current.info.uri.as_deref().unwrap_or(""),
                format_duration(current.info.length)
            ));
            total_duration += current.info.length;
        } else {
            description.push_str("**🔇 Not playing anything.**\n\n");
        }

        let size = tracks.len();
        if size > 0 {
            description.push_str(&format!("**Up Next ({} tracks):**\n", size));

            for (i, track) in tracks.iter().take(10).enumerate() {
                description.push_str(&format!(
                    "`{}.` [{}]({}) | `{}`\n",
                    i + 1,
                    track.info.title,
                    track.info.uri.as_deref().unwrap_or(""),
                    format_duration(track.info.length)
                ));
                total_duration += track.info.length;
            }

            if size > 10 {
                description.push_str(&format!("\n*...and {} more tracks*", size - 10));
            }

            for track in tracks.iter().skip(10) {
                total_duration += track.info.length;
            }
        } else if current.is_some() {
            description.push_str("📭 The queue is empty.");
        } else {
            let embed = EmbedBuilder::new()
                .description("❌ No active player in this server.")
                .color(0xFF0000)
                .build();
            let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            return Ok(());
        }

        let embed = EmbedBuilder::new()
            .description(description)
            .color(module_ctx.embed_color)
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(format!(
                "Total Queue Duration: {}",
                format_duration(total_duration)
            )))
            .build();

        let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
    } else {
        let embed = EmbedBuilder::new()
            .description("❌ Invalid response from audio node.")
            .color(0xFF0000)
            .build();
        let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
    }

    Ok(())
}

pub async fn handle_prefix_queue(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = ctx.guild_id.to_string();
    let req_id = uuid::Uuid::new_v4().to_string();

    let cmd = MusicCommand::Queue { guild_id: guild_id.clone(), req_id: req_id.clone() };

    let payload = serde_json::to_string(&cmd).unwrap_or_default();
    let (tx, rx) = tokio::sync::oneshot::channel();

    if let Some(map) = MUSIC_RESPONSES.get() {
        map.insert(req_id.clone(), tx);
    } else {
        return Err(HarmonyError::Internal("Music responses map not initialized".to_string()));
    }

    {
        use redis::AsyncCommands;
        let mut redis_conn = module_ctx.cache.clone();
        let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;
    }

    let response = match timeout(Duration::from_secs(5), rx).await {
        Ok(Ok(res)) => res,
        _ => {
            if let Some(map) = MUSIC_RESPONSES.get() {
                map.remove(&req_id);
            }
            let embed = EmbedBuilder::new()
                .description("❌ Timeout waiting for queue.")
                .color(0xFF0000)
                .build();
            let _ = ctx.reply_embed(embed, module_ctx).await;
            return Ok(());
        }
    };

    match response {
        MusicResponse::QueueResult { tracks, .. } => {
            if tracks.is_empty() {
                let embed = EmbedBuilder::new()
                    .description("📭 The queue is currently empty.")
                    .color(module_ctx.embed_color)
                    .build();
                let _ = ctx.reply_embed(embed, module_ctx).await;
            } else {
                let mut content = String::new();
                for (i, track) in tracks.iter().take(10).enumerate() {
                    let title = if track.info.title.len() > 50 {
                        format!("{}...", &track.info.title[0..47])
                    } else {
                        track.info.title.clone()
                    };

                    let dur_str = format!(
                        "{:02}:{:02}",
                        track.info.length / 60000,
                        (track.info.length / 1000) % 60
                    );

                    content.push_str(&format!("`{}.` **{}** `[{}]`\n", i + 1, title, dur_str));
                }

                if tracks.len() > 10 {
                    content.push_str(&format!("\n*...and {} more tracks*", tracks.len() - 10));
                }

                let embed = EmbedBuilder::new()
                    .title("🎶 Current Queue")
                    .description(content)
                    .color(module_ctx.embed_color)
                    .build();

                let _ = ctx.reply_embed(embed, module_ctx).await;
            }
        }
        MusicResponse::Error { message, .. } => {
            let embed = EmbedBuilder::new()
                .description(format!("❌ Could not fetch queue: {}", message))
                .color(0xFF0000)
                .build();
            let _ = ctx.reply_embed(embed, module_ctx).await;
        }
        _ => {
            let embed = EmbedBuilder::new()
                .description("❌ Invalid response from audio node.")
                .color(0xFF0000)
                .build();
            let _ = ctx.reply_embed(embed, module_ctx).await;
        }
    }

    Ok(())
}
