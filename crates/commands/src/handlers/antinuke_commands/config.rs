use super::AntinukeCommandHandler;
use chrono::Utc;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::models::antinuke_config::{AntinukeConfig, AntinukeModuleConfigRow};
use railway_database::repository::antinuke_repository::AntinukeRepository;
use twilight_model::id::Id;

impl AntinukeCommandHandler {
    pub(super) async fn handle_enable(
        &self,
        guild_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        let mut config = repo.get_config(guild_id).await?.unwrap_or_else(|| AntinukeConfig {
            guild_id,
            enabled: false,
            log_channel_id: None,
            updated_at: Utc::now(),
        });

        if config.log_channel_id.is_none() {
            let channel = module_ctx
                .discord
                .create_guild_channel(Id::new(guild_id as u64), "railway-logs")
                .await?
                .model()
                .await?;
            config.log_channel_id = Some(channel.id.get() as i64);
        }

        config.enabled = true;
        repo.upsert_config(&config).await?;

        railway_antinuke::reload_guild_config(&module_ctx.db, guild_id as u64).await;

        Ok("✅ AntiNuke is now **enabled** for this server. Log channel created.".to_string())
    }

    pub(super) async fn handle_disable(
        &self,
        guild_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        let mut config = repo.get_config(guild_id).await?.unwrap_or_else(|| AntinukeConfig {
            guild_id,
            enabled: false,
            log_channel_id: None,
            updated_at: Utc::now(),
        });

        config.enabled = false;
        repo.upsert_config(&config).await?;

        railway_antinuke::reload_guild_config(&module_ctx.db, guild_id as u64).await;

        Ok("❌ AntiNuke is now **disabled** for this server.".to_string())
    }

    pub(super) async fn handle_limit(
        &self,
        guild_id: i64,
        action: String,
        threshold: i32,
        window_secs: i32,
        punishment: String,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());

        let module_config = AntinukeModuleConfigRow {
            guild_id,
            action_type: action.clone(),
            enabled: true,
            threshold,
            window_secs,
            punishment: punishment.clone(),
            log_only: false,
        };

        repo.upsert_module_config(&module_config).await?;

        railway_antinuke::reload_guild_config(&module_ctx.db, guild_id as u64).await;

        if threshold == 0 {
            Ok(format!("🚨 **Zero-Tolerance limit set for {}!** Instant {} will be applied on the very first detection.", action, punishment))
        } else {
            Ok(format!("✅ Limit updated for **{}**: **{}** actions allowed within **{}** seconds. Punishment: **{}**", action, threshold, window_secs, punishment))
        }
    }
}
