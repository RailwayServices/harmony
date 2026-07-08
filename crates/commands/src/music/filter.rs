use crate::core::interaction::InteractionContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_modules::LAVENDE_MANAGER;
use lavende::EqBand;
use twilight_util::builder::embed::EmbedBuilder;

pub async fn handle_filter(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let filter_type = interaction_ctx.extract_string_option("type").unwrap_or_default();

    let manager = LAVENDE_MANAGER
        .get()
        .ok_or_else(|| HarmonyError::Internal("Lavende Manager not initialized".to_string()))?;

    if let Some(player) = manager.get_player(&guild_id.to_string()) {
        tracing::info!("[FILTER] Applying filter '{}' in Guild {}", filter_type, guild_id);
        let mut filter_manager = player.filter_manager.write().await;

        match filter_type.as_str() {
            "bassboost" => {
                filter_manager.reset_filters();
                let bands = vec![
                    EqBand { band: 0, gain: 0.2 },
                    EqBand { band: 1, gain: 0.15 },
                    EqBand { band: 2, gain: 0.1 },
                ];
                filter_manager.set_equalizer(bands);
            }
            "nightcore" => {
                filter_manager.reset_filters();
                filter_manager.set_timescale(1.2, 1.2, 1.0);
            }
            "vaporwave" => {
                filter_manager.reset_filters();
                filter_manager.set_timescale(0.8, 0.8, 1.0);
            }
            "studio" => {
                filter_manager.reset_filters();
                let bands = vec![
                    EqBand { band: 0, gain: 0.1 },
                    EqBand { band: 1, gain: 0.05 },
                    EqBand { band: 8, gain: 0.05 },
                    EqBand { band: 9, gain: 0.1 },
                ];
                filter_manager.set_equalizer(bands);
            }
            "8d" => {
                filter_manager.reset_filters();
                filter_manager.toggle_rotation(0.2);
            }
            "tremolo" => {
                filter_manager.reset_filters();
                filter_manager.toggle_tremolo(2.0, 0.5);
            }
            "vibrato" => {
                filter_manager.reset_filters();
                filter_manager.toggle_vibrato(2.0, 0.5);
            }
            "clear" => {
                filter_manager.reset_filters();
            }
            _ => {
                let embed = EmbedBuilder::new()
                    .description("❌ Unknown filter type.")
                    .color(0xFF0000)
                    .build();
                let _ = interaction_ctx.reply_embed(embed, module_ctx).await;
                return Ok(());
            }
        }

        drop(filter_manager);
        player.apply_filters().await;

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

        let mut redis_conn = module_ctx.cache.clone();
        harmony_modules::state_sync::sync_player_state(
            &guild_id.to_string(),
            &player,
            &mut redis_conn,
        )
        .await;

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
