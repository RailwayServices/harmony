use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_common::music_ipc::MusicCommand;
use redis::AsyncCommands;
use twilight_util::builder::embed::EmbedBuilder;

async fn send_music_command(
    cmd: MusicCommand,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let payload = serde_json::to_string(&cmd).unwrap_or_default();
    let mut redis_conn = module_ctx.cache.clone();
    let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;
    Ok(())
}

pub async fn handle_stop(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Stop { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏹️ Stopped playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_skip(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Skip { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏭️ Skipped the current track.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_pause(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Pause { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏸️ Paused playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_resume(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Resume { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("▶️ Resumed playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_volume(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let _guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let _vol = interaction_ctx.extract_integer_option("level").unwrap_or(100);

    let embed = EmbedBuilder::new()
        .description("🔊 Volume control over IPC is not yet supported.")
        .color(0xFF0000)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}


pub async fn handle_prefix_stop(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Stop { guild_id: ctx.guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏹️ Stopped playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_skip(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Skip { guild_id: ctx.guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏭️ Skipped the current track.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_pause(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Pause { guild_id: ctx.guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏸️ Paused playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_resume(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Resume { guild_id: ctx.guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("▶️ Resumed playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_volume(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let embed = EmbedBuilder::new()
        .description("🔊 Volume control over IPC is not yet supported.")
        .color(0xFF0000)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}
