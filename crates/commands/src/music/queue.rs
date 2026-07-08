use crate::core::interaction::InteractionContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_modules::LAVENDE_MANAGER;
use twilight_util::builder::embed::EmbedBuilder;

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

    let manager = LAVENDE_MANAGER
        .get()
        .ok_or_else(|| HarmonyError::Internal("Lavende Manager not initialized".to_string()))?;

    if let Some(player) = manager.get_player(&guild_id.to_string()) {
        tracing::info!("[QUEUE] Displaying queue in Guild {}", guild_id);
        let queue = player.queue.read().await;

        let mut description = String::new();
        let mut total_duration = 0;

        if let Some(current) = &queue.current {
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

        let size = queue.size();
        if size > 0 {
            description.push_str(&format!("**Up Next ({} tracks):**\n", size));

            for (i, track) in queue.tracks.iter().take(10).enumerate() {
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

            for track in queue.tracks.iter().skip(10) {
                total_duration += track.info.length;
            }
        } else if queue.current.is_some() {
            description.push_str("📭 The queue is empty.");
        }

        let embed = EmbedBuilder::new()
            .description(description)
            .color(module_ctx.embed_color)
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(format!(
                "Total Queue Duration: {}",
                format_duration(total_duration)
            )))
            .build();

        let _ = interaction_ctx.reply_embed(embed, module_ctx).await;
    } else {
        let embed = EmbedBuilder::new()
            .description("❌ No active player in this server.")
            .color(0xFF0000)
            .build();
        let _ = interaction_ctx.reply_embed(embed, module_ctx).await;
    }

    Ok(())
}
