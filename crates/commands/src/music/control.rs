use crate::core::interaction::InteractionContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_modules::LAVENDE_MANAGER;
use twilight_util::builder::embed::EmbedBuilder;

pub async fn handle_stop(
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
        tracing::info!("[CONTROL] Stopping playback in Guild {}", guild_id);
        player.stop().await;
        let mut redis_conn = module_ctx.cache.clone();
        harmony_modules::state_sync::sync_player_state(
            &guild_id.to_string(),
            &player,
            &mut redis_conn,
        )
        .await;
        let embed = EmbedBuilder::new()
            .description("⏹️ Stopped playback and cleared the queue.")
            .color(module_ctx.embed_color)
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

pub async fn handle_skip(
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
        tracing::info!("[CONTROL] Skipping track in Guild {}", guild_id);
        player.skip().await;
        let mut redis_conn = module_ctx.cache.clone();
        harmony_modules::state_sync::sync_player_state(
            &guild_id.to_string(),
            &player,
            &mut redis_conn,
        )
        .await;
        let embed = EmbedBuilder::new()
            .description("⏭️ Skipped the current track.")
            .color(module_ctx.embed_color)
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

pub async fn handle_pause(
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
        tracing::info!("[CONTROL] Pausing playback in Guild {}", guild_id);
        player.pause(true).await;
        let mut redis_conn = module_ctx.cache.clone();
        harmony_modules::state_sync::sync_player_state(
            &guild_id.to_string(),
            &player,
            &mut redis_conn,
        )
        .await;
        let embed = EmbedBuilder::new()
            .description("⏸️ Paused playback.")
            .color(module_ctx.embed_color)
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

pub async fn handle_resume(
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
        tracing::info!("[CONTROL] Resuming playback in Guild {}", guild_id);
        player.resume().await;
        let mut redis_conn = module_ctx.cache.clone();
        harmony_modules::state_sync::sync_player_state(
            &guild_id.to_string(),
            &player,
            &mut redis_conn,
        )
        .await;
        let embed = EmbedBuilder::new()
            .description("▶️ Resumed playback.")
            .color(module_ctx.embed_color)
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

pub async fn handle_volume(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let vol = interaction_ctx.extract_integer_option("level").unwrap_or(100);

    let manager = LAVENDE_MANAGER
        .get()
        .ok_or_else(|| HarmonyError::Internal("Lavende Manager not initialized".to_string()))?;

    if let Some(player) = manager.get_player(&guild_id.to_string()) {
        tracing::info!("[CONTROL] Setting volume to {} in Guild {}", vol, guild_id);
        player.set_volume(vol as u32).await;
        let mut redis_conn = module_ctx.cache.clone();
        harmony_modules::state_sync::sync_player_state(
            &guild_id.to_string(),
            &player,
            &mut redis_conn,
        )
        .await;
        let embed = EmbedBuilder::new()
            .description(format!("🔊 Volume set to {}%", vol))
            .color(module_ctx.embed_color)
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
