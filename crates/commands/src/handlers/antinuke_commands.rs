use crate::interaction::InteractionContext;
use chrono::Utc;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::models::antinuke_config::{AntinukeConfig, AntinukeModuleConfigRow};
use railway_database::repository::antinuke_repository::AntinukeRepository;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_model::id::Id;
use twilight_util::builder::InteractionResponseDataBuilder;

#[derive(Clone)]
pub struct AntinukeCommandHandler {}

impl AntinukeCommandHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle(
        &self,
        interaction_ctx: &InteractionContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        let interaction = &interaction_ctx.interaction;
        let guild_id = interaction
            .guild_id
            .ok_or_else(|| RailwayError::Internal("Command must be run in a guild".to_string()))?;

        let data = match &interaction.data {
            Some(InteractionData::ApplicationCommand(data)) => data,
            _ => return Err(RailwayError::Internal("Not an application command".to_string())),
        };

        if data.options.is_empty() {
            return Ok(());
        }

        let subcommand = &data.options[0];

        let reply_msg =
            match (subcommand.name.as_str(), &subcommand.value) {
                ("enable", CommandOptionValue::SubCommand(_)) => {
                    self.handle_enable(guild_id.get() as i64, module_ctx).await?
                }
                ("disable", CommandOptionValue::SubCommand(_)) => {
                    self.handle_disable(guild_id.get() as i64, module_ctx).await?
                }
                ("limit", CommandOptionValue::SubCommand(options)) => {
                    let action = options
                        .iter()
                        .find(|o| o.name == "action")
                        .and_then(|o| match &o.value {
                            CommandOptionValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "BAN_ADD".to_string());

                    let threshold = options
                        .iter()
                        .find(|o| o.name == "threshold")
                        .and_then(|o| match &o.value {
                            CommandOptionValue::Integer(i) => Some(*i as i32),
                            _ => None,
                        })
                        .unwrap_or(3);

                    let window_secs = options
                        .iter()
                        .find(|o| o.name == "window_secs")
                        .and_then(|o| match &o.value {
                            CommandOptionValue::Integer(i) => Some(*i as i32),
                            _ => None,
                        })
                        .unwrap_or(10);

                    let punishment = options
                        .iter()
                        .find(|o| o.name == "punishment")
                        .and_then(|o| match &o.value {
                            CommandOptionValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "BAN".to_string());

                    self.handle_limit(
                        guild_id.get() as i64,
                        action,
                        threshold,
                        window_secs,
                        punishment,
                        module_ctx,
                    )
                    .await?
                }
                ("settings", CommandOptionValue::SubCommand(_)) => {
                    self.handle_settings(guild_id.get() as i64, module_ctx).await?
                }
                ("whitelist", CommandOptionValue::SubCommandGroup(group_options)) => {
                    if group_options.is_empty() {
                        "Invalid whitelist subcommand".to_string()
                    } else {
                        let action_opt = &group_options[0];
                        match (action_opt.name.as_str(), &action_opt.value) {
                            ("add", CommandOptionValue::SubCommand(opts)) => {
                                let user = opts.iter().find(|o| o.name == "user").and_then(|o| {
                                    match &o.value {
                                        CommandOptionValue::User(u) => Some(*u),
                                        _ => None,
                                    }
                                });
                                if let Some(uid) = user {
                                    self.handle_whitelist_add(
                                        guild_id.get() as i64,
                                        uid.get() as i64,
                                        interaction.author_id().unwrap().get() as i64,
                                        module_ctx,
                                    )
                                    .await?
                                } else {
                                    "User not found".to_string()
                                }
                            }
                            ("remove", CommandOptionValue::SubCommand(opts)) => {
                                let user = opts.iter().find(|o| o.name == "user").and_then(|o| {
                                    match &o.value {
                                        CommandOptionValue::User(u) => Some(*u),
                                        _ => None,
                                    }
                                });
                                if let Some(uid) = user {
                                    self.handle_whitelist_remove(
                                        guild_id.get() as i64,
                                        uid.get() as i64,
                                        module_ctx,
                                    )
                                    .await?
                                } else {
                                    "User not found".to_string()
                                }
                            }
                            _ => "Unknown whitelist subcommand".to_string(),
                        }
                    }
                }
                _ => "Unknown subcommand".to_string(),
            };

        let interaction_client = module_ctx.discord.interaction(interaction.application_id);

        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseDataBuilder::new().content(reply_msg).build()),
        };

        interaction_client.create_response(interaction.id, &interaction.token, &response).await?;

        Ok(())
    }

    pub async fn handle_prefix(
        &self,
        ctx: &crate::prefix::PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        let guild_id = ctx.guild_id.get() as i64;
        let args = &ctx.args;

        if args.is_empty() {
            return ctx
                .reply("Available commands: `enable`, `disable`, `settings`, `limit`, `whitelist`")
                .await;
        }

        let subcommand = args[0].to_lowercase();
        let reply_msg = match subcommand.as_str() {
            "enable" => self.handle_enable(guild_id, module_ctx).await?,
            "disable" => self.handle_disable(guild_id, module_ctx).await?,
            "settings" => self.handle_settings(guild_id, module_ctx).await?,
            "limit" => {
                let action = args.get(1).cloned().unwrap_or_else(|| "BAN_ADD".to_string());
                let threshold = args.get(2).and_then(|s| s.parse::<i32>().ok()).unwrap_or(3);
                let window = args.get(3).and_then(|s| s.parse::<i32>().ok()).unwrap_or(10);
                let punishment = args.get(4).cloned().unwrap_or_else(|| "BAN".to_string());
                self.handle_limit(guild_id, action, threshold, window, punishment, module_ctx)
                    .await?
            }
            "whitelist" => {
                let sub = args.get(1).map(|s| s.to_lowercase());
                let target = args.get(2).and_then(|s| {
                    let cleaned = s.replace("<@", "").replace(">", "").replace("!", "");
                    cleaned.parse::<u64>().ok()
                });

                if let (Some(sub), Some(uid)) = (sub, target) {
                    if sub == "add" {
                        self.handle_whitelist_add(
                            guild_id,
                            uid as i64,
                            ctx.message.author.id.get() as i64,
                            module_ctx,
                        )
                        .await?
                    } else if sub == "remove" {
                        self.handle_whitelist_remove(guild_id, uid as i64, module_ctx).await?
                    } else {
                        "Unknown whitelist subcommand".to_string()
                    }
                } else {
                    "Invalid whitelist arguments. Use: `whitelist add @user` or `whitelist remove @user`".to_string()
                }
            }
            _ => "Unknown antinuke command.".to_string(),
        };

        ctx.reply(&reply_msg).await?;
        Ok(())
    }

    async fn handle_enable(
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

    async fn handle_disable(
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

    async fn handle_limit(
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

    async fn handle_settings(
        &self,
        guild_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        let config = repo.get_config(guild_id).await?;
        let modules = repo.get_module_configs(guild_id).await?;
        let wl_count = repo.get_whitelist_count(guild_id).await?;

        if let Some(c) = config {
            let status = if c.enabled { "🟢 Enabled" } else { "🔴 Disabled" };
            let log_ch = c
                .log_channel_id
                .map(|id| format!("<#{}>", id))
                .unwrap_or_else(|| "None".to_string());

            let mut mods_str = String::new();
            for m in modules {
                mods_str.push_str(&format!(
                    "- {}: {}/{}s ({})\n",
                    m.action_type, m.threshold, m.window_secs, m.punishment
                ));
            }
            if mods_str.is_empty() {
                mods_str = "No specific limits configured.".to_string();
            }

            Ok(format!("**AntiNuke Settings**\nStatus: {}\nLog Channel: {}\nWhitelisted Users: {}\n\n**Limits:**\n{}", status, log_ch, wl_count, mods_str))
        } else {
            Ok(format!(
                "**AntiNuke Settings**\nStatus: 🔴 Disabled\nWhitelisted Users: {}",
                wl_count
            ))
        }
    }

    async fn handle_whitelist_add(
        &self,
        guild_id: i64,
        user_id: i64,
        added_by: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        repo.add_whitelist(guild_id, user_id, added_by).await?;

        railway_antinuke::whitelist_add(guild_id as u64, user_id as u64);

        Ok(format!("✅ User <@{}> added to whitelist.", user_id))
    }

    async fn handle_whitelist_remove(
        &self,
        guild_id: i64,
        user_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        repo.remove_whitelist(guild_id, user_id).await?;

        railway_antinuke::whitelist_remove(guild_id as u64, user_id as u64);

        Ok(format!("✅ User <@{}> removed from whitelist.", user_id))
    }
}

impl Default for AntinukeCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
