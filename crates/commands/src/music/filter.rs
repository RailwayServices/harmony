use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_common::music_ipc::MusicCommand;
use redis::AsyncCommands;
use twilight_util::builder::embed::EmbedBuilder;

pub async fn handle_filter(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let filter_type = interaction_ctx.extract_string_option("type").unwrap_or_default();

    let valid_filters =
        ["bassboost", "nightcore", "vaporwave", "studio", "8d", "tremolo", "vibrato", "clear"];

    if !valid_filters.contains(&filter_type.as_str()) {
        let embed =
            EmbedBuilder::new().description("❌ Unknown filter type.").color(0xFF0000).build();
        let _ = interaction_ctx.reply_embed(embed, module_ctx).await;
        return Ok(());
    }

    let cmd =
        MusicCommand::Filter { guild_id: guild_id.to_string(), filter_type: filter_type.clone() };

    let payload = serde_json::to_string(&cmd).unwrap_or_default();
    let mut redis_conn = module_ctx.cache.clone();
    let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;

    let emoji = if filter_type == "clear" { "🧹" } else { "🎛️" };
    let msg = if filter_type == "clear" {
        "Cleared all audio filters.".to_string()
    } else {
        format!("Applied **{}** filter.", filter_type)
    };

    let embed = EmbedBuilder::new()
        .description(format!("{} {}", emoji, msg))
        .color(module_ctx.embed_color)
        .build();

    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}


pub async fn handle_prefix_filter(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let filter = ctx.args.join(" ");

    let filter_type = filter.to_lowercase();
    let valid_filters = ["nightcore", "bassboost", "vaporwave", "8d", "karaoke", "tremolo", "vibrato", "pop", "soft", "treblebass", "echo", "chorus", "flanger", "gate", "haas", "phaser", "compressor", "expander", "lowpass", "highpass", "none", "clear", "reset", ""];
    
    if !valid_filters.contains(&filter_type.as_str()) {
        let embed = EmbedBuilder::new()
            .description("❌ Unknown filter!")
            .color(0xFF0000)
            .build();
        let _ = ctx.reply_embed(embed, module_ctx).await;
        return Ok(());
    }

    let payload = serde_json::to_string(&MusicCommand::Filter {
        guild_id: ctx.guild_id.to_string(),
        filter_type,
    })
    .unwrap_or_default();

    {
        use redis::AsyncCommands;
        let mut redis_conn = module_ctx.cache.clone();
        let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;
    }

    let msg = if filter.is_empty() || filter == "none" {
        "🎛️ Cleared all audio filters".to_string()
    } else {
        format!("🎛️ Applied filter: **{}**", filter)
    };

    let embed = EmbedBuilder::new().description(msg).color(module_ctx.embed_color).build();
    let _ = ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}
