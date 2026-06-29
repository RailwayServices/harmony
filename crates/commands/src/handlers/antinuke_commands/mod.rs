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

        let interaction_client = module_ctx.discord.interaction(interaction.application_id);

        let embed = railway_common::ui::build_stylish_embed(
            "AntiNuke Command",
            &reply_msg,
            module_ctx.embed_color,
        );
        let action_row = railway_common::ui::build_support_action_row();

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
