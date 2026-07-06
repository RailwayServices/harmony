use super::AntinukeCommandHandler;
use chrono::Utc;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::models::antinuke_config::{AntinukeConfig, AntinukeModuleConfigRow};
use railway_database::repository::antinuke_repository::AntinukeRepository;
use twilight_model::channel::permission_overwrite::{PermissionOverwrite, PermissionOverwriteType};
use twilight_model::guild::Permissions;
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

        if config.enabled {
            return Ok("✅ AntiNuke is already enabled for this server.".to_string());
        }

        if config.log_channel_id.is_none() {
            let everyone_overwrite = PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                id: Id::new(guild_id as u64),
                kind: PermissionOverwriteType::Role,
            };

            let channel = module_ctx
                .discord
                .create_guild_channel(Id::new(guild_id as u64), "railway-logs")
                .permission_overwrites(&[everyone_overwrite])
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

        if !config.enabled {
            return Ok("❌ AntiNuke is already disabled for this server.".to_string());
        }

        if let Some(channel_id) = config.log_channel_id {
            let _ = module_ctx.discord.delete_channel(Id::new(channel_id as u64)).await;
            config.log_channel_id = None;
        }

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
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());

        let configs = repo.get_module_configs(guild_id).await?;
        let existing = configs.into_iter().find(|c| c.action_type == action);
        let punishment = existing.map(|c| c.punishment).unwrap_or_else(|| "BAN".to_string());

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
            Ok(format!(
                "✅ Limit updated for **{}**: **{}** actions allowed. Punishment: **{}**",
                action, threshold, punishment
            ))
        }
    }

    pub(super) async fn handle_punishment(
        &self,
        guild_id: i64,
        action: String,
        punishment: String,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());

        let configs = repo.get_module_configs(guild_id).await?;
        let mut existing =
            configs.into_iter().find(|c| c.action_type == action).unwrap_or_else(|| {
                AntinukeModuleConfigRow {
                    guild_id,
                    action_type: action.clone(),
                    enabled: false,
                    threshold: 3,
                    window_secs: 60,
                    punishment: "BAN".to_string(),
                    log_only: false,
                }
            });

        existing.punishment = punishment.clone();
        repo.upsert_module_config(&existing).await?;

        railway_antinuke::reload_guild_config(&module_ctx.db, guild_id as u64).await;

        Ok(format!("⚖️ Punishment for **{}** has been updated to **{}**.", action, punishment))
    }
}
