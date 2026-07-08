use crate::core::interaction::InteractionContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_modules::{LAVENDE_MANAGER, VOICE_STATES};
use lavende::LoadResult;
use twilight_util::builder::embed::EmbedBuilder;

pub async fn handle(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let user_id = interaction_ctx
        .user_id
        .ok_or_else(|| HarmonyError::Internal("Could not extract user id".to_string()))?;

    let query = interaction_ctx.extract_string_option("query").unwrap_or_default();

    if query.is_empty() {
        let embed = EmbedBuilder::new()
            .description("❌ Please provide a search query or URL.")
            .color(0xFF0000)
            .build();
        return interaction_ctx.reply_embed(embed, module_ctx).await;
    }

    let channel_id = {
        let states = VOICE_STATES.get().ok_or_else(|| {
            HarmonyError::Internal("Voice states cache not initialized".to_string())
        })?;

        match states.get(&user_id.get()) {
            Some(id) => *id,
            None => {
                let embed = EmbedBuilder::new()
                    .description("❌ You must be in a voice channel!")
                    .color(0xFF0000)
                    .build();
                return interaction_ctx.reply_embed(embed, module_ctx).await;
            }
        }
    };

    interaction_ctx.defer(module_ctx).await?;

    let search_query =
        if query.starts_with("http") { query.clone() } else { format!("ytsearch:{}", query) };

    let manager = LAVENDE_MANAGER
        .get()
        .ok_or_else(|| HarmonyError::Internal("Lavende Manager not initialized".to_string()))?;

    let player = manager.get_or_create_player(&guild_id.to_string());

    let ((), search_result) = tokio::join!(
        player.connect(Some(channel_id.to_string()), true, false),
        player.search(&search_query)
    );

    let text_channel_id = interaction_ctx.interaction.channel.as_ref().map(|c| c.id);

    match search_result {
        Ok(result) => match result {
            LoadResult::Empty {} => {
                let embed =
                    EmbedBuilder::new().description("❌ No results found.").color(0xFF0000).build();
                let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            }
            LoadResult::Track(track) => {
                tracing::info!("[PLAY] Track: {} by {}", track.info.title, track.info.author);
                let should_play = {
                    let mut q = player.queue.write().await;
                    let was_empty = q.current.is_none() && q.tracks.is_empty();
                    q.add(track.clone());
                    was_empty
                };
                let embed = EmbedBuilder::new()
                    .description(format!("✅ **{}** by {}", track.info.title, track.info.author))
                    .color(module_ctx.embed_color)
                    .build();
                let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
                if should_play {
                    if let Some(ch) = text_channel_id {
                        player.set_data("text_channel_id", serde_json::json!(ch.get()));
                    }
                    let _ = player.play().await;
                }
                let mut redis_conn = module_ctx.cache.clone();
                harmony_modules::state_sync::sync_player_state(
                    &guild_id.to_string(),
                    &player,
                    &mut redis_conn,
                )
                .await;
            }
            LoadResult::Search(tracks) => {
                if let Some(track) = tracks.first() {
                    tracing::info!("[PLAY] Track: {} by {}", track.info.title, track.info.author);
                    let should_play = {
                        let mut q = player.queue.write().await;
                        let was_empty = q.current.is_none() && q.tracks.is_empty();
                        q.add(track.clone());
                        was_empty
                    };
                    let embed = EmbedBuilder::new()
                        .description(format!(
                            "✅ **{}** by {}",
                            track.info.title, track.info.author
                        ))
                        .color(module_ctx.embed_color)
                        .build();
                    let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
                    if should_play {
                        if let Some(ch) = text_channel_id {
                            player.set_data("text_channel_id", serde_json::json!(ch.get()));
                        }
                        let _ = player.play().await;
                    }
                    let mut redis_conn = module_ctx.cache.clone();
                    harmony_modules::state_sync::sync_player_state(
                        &guild_id.to_string(),
                        &player,
                        &mut redis_conn,
                    )
                    .await;
                }
            }
            LoadResult::Playlist(playlist) => {
                let count = playlist.tracks.len();
                tracing::info!("[PLAY] Playlist: {} tracks", count);
                let should_play = {
                    let mut q = player.queue.write().await;
                    let was_empty = q.current.is_none() && q.tracks.is_empty();
                    q.add_multiple(playlist.tracks);
                    was_empty
                };
                let embed = EmbedBuilder::new()
                    .description(format!("📃 Added **{}** tracks from playlist", count))
                    .color(module_ctx.embed_color)
                    .build();
                let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
                if should_play {
                    if let Some(ch) = text_channel_id {
                        player.set_data("text_channel_id", serde_json::json!(ch.get()));
                    }
                    let _ = player.play().await;
                }
                let mut redis_conn = module_ctx.cache.clone();
                harmony_modules::state_sync::sync_player_state(
                    &guild_id.to_string(),
                    &player,
                    &mut redis_conn,
                )
                .await;
            }
            LoadResult::Error(e) => {
                tracing::error!("[PLAY] Load error: {:?}", e.message);
                let embed = EmbedBuilder::new()
                    .description(format!("❌ Error loading track: {:?}", e.message))
                    .color(0xFF0000)
                    .build();
                let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            }
        },
        Err(e) => {
            tracing::error!("[PLAY] Search failed: {}", e);
            let embed = EmbedBuilder::new()
                .description(format!("❌ Search failed: {}", e))
                .color(0xFF0000)
                .build();
            let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
        }
    }

    Ok(())
}
