use crate::interaction::InteractionContext;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

pub mod config;
pub mod settings;
pub mod whitelist;

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

        let reply_msg = match (subcommand.name.as_str(), &subcommand.value) {
            ("enable", CommandOptionValue::SubCommand(_)) => {
                self.handle_enable(guild_id.get() as i64, module_ctx).await?
            }
            ("disable", CommandOptionValue::SubCommand(_)) => {
                self.handle_disable(guild_id.get() as i64, module_ctx).await?
            }
            ("set", CommandOptionValue::SubCommandGroup(group_options)) => {
                if group_options.is_empty() {
                    "Invalid set subcommand".to_string()
                } else {
                    let action_opt = &group_options[0];
                    if action_opt.name == "limit" {
                        if let CommandOptionValue::SubCommand(options) = &action_opt.value {
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
                                .find(|o| o.name == "limit")
                                .and_then(|o| match &o.value {
                                    CommandOptionValue::Integer(i) => Some(*i as i32),
                                    _ => None,
                                })
                                .unwrap_or(0);

                            let window_secs = 60; // Default window
                            let punishment = "BAN".to_string(); // Default punishment

                            self.handle_limit(
                                guild_id.get() as i64,
                                action,
                                threshold,
                                window_secs,
                                punishment,
                                module_ctx,
                            )
                            .await?
                        } else {
                            "Invalid limit options".to_string()
                        }
                    } else {
                        "Unknown set subcommand".to_string()
                    }
                }
            }
            ("whitelisted", CommandOptionValue::SubCommand(_)) => {
                let repo =
                    railway_database::repository::antinuke_repository::AntinukeRepository::new(
                        module_ctx.db.clone(),
                    );
                let list = repo.get_whitelist(guild_id.get() as i64).await.unwrap_or_default();
                let count = list.len();

                if count == 0 {
                    "🛡️ There are currently **0** users whitelisted from AntiNuke.".to_string()
                } else {
                    let mentions: Vec<String> =
                        list.into_iter().map(|id| format!("<@{}>", id)).collect();
                    let mut text = format!(
                        "🛡️ There are currently **{}** users whitelisted from AntiNuke:\n",
                        count
                    );

                    let joined = mentions.join(", ");
                    if joined.len() > 1800 {
                        text.push_str(&joined[..1800]);
                        text.push_str("... and more");
                    } else {
                        text.push_str(&joined);
                    }
                    text
                }
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
                            let user =
                                opts.iter().find(|o| o.name == "user").and_then(|o| {
                                    match &o.value {
                                        CommandOptionValue::User(u) => Some(*u),
                                        _ => None,
                                    }
                                });
                            if let Some(uid) = user {
                                let author_id = interaction.author_id().ok_or_else(|| {
                                    RailwayError::Internal("Interaction has no author".to_string())
                                })?;
                                self.handle_whitelist_add(
                                    guild_id.get() as i64,
                                    uid.get() as i64,
                                    author_id.get() as i64,
                                    module_ctx,
                                )
                                .await?
                            } else {
                                "User not found".to_string()
                            }
                        }
                        ("remove", CommandOptionValue::SubCommand(opts)) => {
                            let user =
                                opts.iter().find(|o| o.name == "user").and_then(|o| {
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

        let action_row = if subcommand.name.as_str() == "settings" {
            let repo = railway_database::repository::antinuke_repository::AntinukeRepository::new(
                module_ctx.db.clone(),
            );
            let config = repo.get_config(guild_id.get() as i64).await?;
            let enabled = config.map(|c| c.enabled).unwrap_or(false);
            railway_common::ui::build_antinuke_settings_buttons(enabled)
        } else {
            railway_common::ui::build_support_action_row()
        };

        let interaction_client = module_ctx.discord.interaction(interaction.application_id);

        let embed = railway_common::ui::build_stylish_embed(
            if subcommand.name.as_str() == "settings" {
                "AntiNuke Settings"
            } else {
                "AntiNuke Command"
            },
            &reply_msg,
            module_ctx.embed_color,
        );

        let response = InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(
                InteractionResponseDataBuilder::new()
                    .embeds([embed])
                    .components([action_row])
                    .build(),
            ),
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
            let embed = railway_common::ui::build_stylish_embed(
                "AntiNuke Help",
                "Available commands: `enable`, `disable`, `settings`, `limit`, `whitelist`",
                module_ctx.embed_color,
            );
            let action_row = railway_common::ui::build_support_action_row();
            return ctx.reply_with_ui(embed, vec![action_row]).await;
        }

        let subcommand = args[0].to_lowercase();
        let reply_msg = match subcommand.as_str() {
            "enable" => self.handle_enable(guild_id, module_ctx).await?,
            "disable" => self.handle_disable(guild_id, module_ctx).await?,
            "settings" => self.handle_settings(guild_id, module_ctx).await?,
            "set" => {
                let sub = args.get(1).map(|s| s.to_lowercase());
                if sub.as_deref() == Some("limit") {
                    let action = args.get(2).cloned().unwrap_or_else(|| "BAN_ADD".to_string());
                    let threshold = args.get(3).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                    let window = 60;
                    let punishment = "BAN".to_string();
                    self.handle_limit(guild_id, action, threshold, window, punishment, module_ctx)
                        .await?
                } else {
                    "Invalid set subcommand. Use: `set limit`".to_string()
                }
            }
            "whitelisted" => {
                let repo =
                    railway_database::repository::antinuke_repository::AntinukeRepository::new(
                        module_ctx.db.clone(),
                    );
                let list = repo.get_whitelist(guild_id).await.unwrap_or_default();
                let count = list.len();

                if count == 0 {
                    "🛡️ There are currently **0** users whitelisted from AntiNuke.".to_string()
                } else {
                    let mentions: Vec<String> =
                        list.into_iter().map(|id| format!("<@{}>", id)).collect();
                    let mut text = format!(
                        "🛡️ There are currently **{}** users whitelisted from AntiNuke:\n",
                        count
                    );

                    let joined = mentions.join(", ");
                    if joined.len() > 1800 {
                        text.push_str(&joined[..1800]);
                        text.push_str("... and more");
                    } else {
                        text.push_str(&joined);
                    }
                    text
                }
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

        let embed = railway_common::ui::build_stylish_embed(
            "AntiNuke Command",
            &reply_msg,
            module_ctx.embed_color,
        );
        let action_row = railway_common::ui::build_support_action_row();

        ctx.reply_with_ui(embed, vec![action_row]).await?;
        Ok(())
    }
}

impl Default for AntinukeCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AntinukeCommandHandler {
    pub async fn handle_settings_wrapper(
        &self,
        guild_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        self.handle_settings(guild_id, module_ctx).await
    }
}
