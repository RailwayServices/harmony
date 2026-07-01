use crate::handlers::antinuke_commands::AntinukeCommandHandler;
use crate::handlers::automod_commands::AutomodCommandHandler;
use crate::interaction::InteractionContext;
use crate::prefix::PrefixRouter;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;

pub struct CommandRouter {
    antinuke_handler: AntinukeCommandHandler,
    automod_handler: AutomodCommandHandler,
    prefix_router: PrefixRouter,
}

impl CommandRouter {
    pub fn new(prefix: String) -> Self {
        Self {
            antinuke_handler: AntinukeCommandHandler::new(),
            automod_handler: AutomodCommandHandler::new(),
            prefix_router: PrefixRouter::new(prefix),
        }
    }

    pub async fn handle_event(
        &self,
        event: &railway_common::event::RailwayEvent,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        if let railway_common::event::RailwayEvent::Discord(box_event) = event {
            match &**box_event {
                twilight_model::gateway::event::Event::InteractionCreate(interaction) => {
                    let interaction_ctx = InteractionContext::new(interaction.0.clone());
                    return self.route(&interaction_ctx, module_ctx).await;
                }
                twilight_model::gateway::event::Event::MessageCreate(msg) => {
                    return self.prefix_router.handle_message(&msg.0, module_ctx).await;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn route(
        &self,
        interaction_ctx: &InteractionContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        if interaction_ctx.is_component() {
            let custom_id = interaction_ctx.extract_custom_id()?;
            return self.handle_component_interaction(interaction_ctx, custom_id, module_ctx).await;
        }

        let name = interaction_ctx.extract_command_name()?;

        match name {
            "antinuke" => self.antinuke_handler.handle(interaction_ctx, module_ctx).await,
            "automod" => self.automod_handler.handle(interaction_ctx, module_ctx).await,
            _ => Err(RailwayError::Internal(format!("Unknown command: {}", name))),
        }
    }

    pub async fn handle_component_interaction(
        &self,
        interaction_ctx: &InteractionContext,
        custom_id: &str,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        let interaction = &interaction_ctx.interaction;
        let guild_id = interaction
            .guild_id
            .ok_or_else(|| RailwayError::Internal("Interaction must be in a guild".to_string()))?;

        let mut has_perm = false;
        if let Some(member) = &interaction.member {
            if let Some(perms) = member.permissions {
                has_perm = perms.contains(twilight_model::guild::Permissions::MANAGE_GUILD)
                    || perms.contains(twilight_model::guild::Permissions::ADMINISTRATOR);
            }
        }

        let interaction_client = module_ctx.discord.interaction(interaction.application_id);

        if !has_perm {
            let response = twilight_model::http::interaction::InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    twilight_util::builder::InteractionResponseDataBuilder::new()
                        .content("❌ Aapke paas is action ko perform karne ki permissions (Manage Guild) nahi hain!")
                        .flags(twilight_model::channel::message::MessageFlags::EPHEMERAL)
                        .build()
                )
            };
            interaction_client
                .create_response(interaction.id, &interaction.token, &response)
                .await?;
            return Ok(());
        }

        if custom_id.starts_with("antinuke_wl:") {
            let target_user_id_str = custom_id.trim_start_matches("antinuke_wl:");
            let target_user_id = target_user_id_str.parse::<u64>().map_err(|_| {
                RailwayError::Internal("Invalid target user ID in custom_id".to_string())
            })?;

            let clicker_id = interaction
                .author_id()
                .ok_or_else(|| RailwayError::Internal("Interaction has no author".to_string()))?;

            let repo = railway_database::repository::antinuke_repository::AntinukeRepository::new(
                module_ctx.db.clone(),
            );
            repo.add_whitelist(
                guild_id.get() as i64,
                target_user_id as i64,
                clicker_id.get() as i64,
            )
            .await?;
            railway_antinuke::whitelist_add(guild_id.get(), target_user_id);

            let response = twilight_model::http::interaction::InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                data: Some(
                    twilight_util::builder::InteractionResponseDataBuilder::new()
                        .content(format!("✅ User <@{}> has been whitelisted successfully!", target_user_id))
                        .flags(twilight_model::channel::message::MessageFlags::EPHEMERAL)
                        .build()
                )
            };
            interaction_client
                .create_response(interaction.id, &interaction.token, &response)
                .await?;
        } else if custom_id.starts_with("antinuke_unban:") {
            let target_user_id_str = custom_id.trim_start_matches("antinuke_unban:");
            let target_user_id = target_user_id_str.parse::<u64>().map_err(|_| {
                RailwayError::Internal("Invalid target user ID in custom_id".to_string())
            })?;

            let unban_fut = module_ctx
                .discord
                .delete_ban(guild_id, twilight_model::id::Id::new(target_user_id));
            match unban_fut.await {
                Ok(_) => {
                    let response = twilight_model::http::interaction::InteractionResponse {
                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                        data: Some(
                            twilight_util::builder::InteractionResponseDataBuilder::new()
                                .content(format!("✅ User <@{}> has been unbanned successfully!", target_user_id))
                                .flags(twilight_model::channel::message::MessageFlags::EPHEMERAL)
                                .build()
                        )
                    };
                    interaction_client
                        .create_response(interaction.id, &interaction.token, &response)
                        .await?;
                }
                Err(e) => {
                    let response = twilight_model::http::interaction::InteractionResponse {
                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                        data: Some(
                            twilight_util::builder::InteractionResponseDataBuilder::new()
                                .content(format!("❌ Failed to unban user: {}", e))
                                .flags(twilight_model::channel::message::MessageFlags::EPHEMERAL)
                                .build()
                        )
                    };
                    interaction_client
                        .create_response(interaction.id, &interaction.token, &response)
                        .await?;
                }
            }
        } else if custom_id == "antinuke_toggle" {
            let repo = railway_database::repository::antinuke_repository::AntinukeRepository::new(
                module_ctx.db.clone(),
            );
            let mut config = repo.get_config(guild_id.get() as i64).await?.unwrap_or_else(|| {
                railway_database::models::antinuke_config::AntinukeConfig {
                    guild_id: guild_id.get() as i64,
                    enabled: false,
                    log_channel_id: None,
                    updated_at: chrono::Utc::now(),
                }
            });

            config.enabled = !config.enabled;

            if config.enabled && config.log_channel_id.is_none() {
                let channel = module_ctx
                    .discord
                    .create_guild_channel(guild_id, "railway-logs")
                    .await?
                    .model()
                    .await?;
                config.log_channel_id = Some(channel.id.get() as i64);
            }

            let new_state = config.enabled;
            repo.upsert_config(&config).await?;
            railway_antinuke::reload_guild_config(&module_ctx.db, guild_id.get()).await;

            let settings_str = self
                .antinuke_handler
                .handle_settings_wrapper(guild_id.get() as i64, module_ctx)
                .await?;

            let embed = railway_common::ui::build_stylish_embed(
                "AntiNuke Settings",
                &settings_str,
                module_ctx.embed_color,
            );
            let action_row = railway_common::ui::build_antinuke_settings_buttons(new_state);

            let response = twilight_model::http::interaction::InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::UpdateMessage,
                data: Some(
                    twilight_util::builder::InteractionResponseDataBuilder::new()
                        .embeds([embed])
                        .components([action_row])
                        .build(),
                ),
            };
            interaction_client
                .create_response(interaction.id, &interaction.token, &response)
                .await?;
        } else if custom_id.starts_with("automod_toggle:") {
            let filter = custom_id.trim_start_matches("automod_toggle:");
            let trigger_type = match filter {
                "spam" => 1,
                "antilink" => 2,
                "ghostping" => 3,
                _ => return Err(RailwayError::Internal("Invalid filter type".to_string())),
            };

            let repo =
                railway_database::repository::automod_repository::AutomodRepository::get_rule(
                    &module_ctx.db,
                    guild_id.get() as i64,
                    trigger_type,
                )
                .await?;
            if let Some(mut r) = repo {
                r.enabled = !r.enabled;
                railway_database::repository::automod_repository::AutomodRepository::update_rule(
                    &module_ctx.db,
                    &r,
                )
                .await?;
            } else {
                let default_action = match trigger_type {
                    1 => 2, // Timeout
                    2 => 1, // DeleteMessage
                    3 => 2, // Timeout
                    _ => 1,
                };
                railway_database::repository::automod_repository::AutomodRepository::create_rule(
                    &module_ctx.db,
                    guild_id.get() as i64,
                    &format!("AutoMod {:?}", filter),
                    trigger_type,
                    default_action,
                    true,
                )
                .await?;
            };

            let settings_str =
                self.automod_handler.handle_settings(guild_id.get() as i64, module_ctx).await?;

            let spam =
                railway_database::repository::automod_repository::AutomodRepository::get_rule(
                    &module_ctx.db,
                    guild_id.get() as i64,
                    1,
                )
                .await?
                .map(|r| r.enabled)
                .unwrap_or(false);
            let antilink =
                railway_database::repository::automod_repository::AutomodRepository::get_rule(
                    &module_ctx.db,
                    guild_id.get() as i64,
                    2,
                )
                .await?
                .map(|r| r.enabled)
                .unwrap_or(false);
            let ghostping =
                railway_database::repository::automod_repository::AutomodRepository::get_rule(
                    &module_ctx.db,
                    guild_id.get() as i64,
                    3,
                )
                .await?
                .map(|r| r.enabled)
                .unwrap_or(false);

            let embed = railway_common::ui::build_stylish_embed(
                "AutoMod Settings",
                &settings_str,
                module_ctx.embed_color,
            );
            let action_row =
                railway_common::ui::build_automod_settings_buttons(spam, antilink, ghostping);

            let response = twilight_model::http::interaction::InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::UpdateMessage,
                data: Some(
                    twilight_util::builder::InteractionResponseDataBuilder::new()
                        .embeds([embed])
                        .components([action_row])
                        .build(),
                ),
            };
            interaction_client
                .create_response(interaction.id, &interaction.token, &response)
                .await?;
        }

        Ok(())
    }
}
