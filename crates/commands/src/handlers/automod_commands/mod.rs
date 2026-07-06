use crate::interaction::InteractionContext;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

pub mod config;
mod words;

#[derive(Clone)]
pub struct AutomodCommandHandler {}

impl AutomodCommandHandler {
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
            ("settings", CommandOptionValue::SubCommand(_)) => {
                "AutoMod settings are now managed in Discord's native Server Settings -> AutoMod."
                    .to_string()
            }
            _ => "Unknown subcommand".to_string(),
        };

        let action_row = railway_common::ui::build_support_action_row();

        let interaction_client = module_ctx.discord.interaction(interaction.application_id);

        let embed = railway_common::ui::build_stylish_embed(
            if subcommand.name.as_str() == "settings" {
                "AutoMod Settings"
            } else {
                "AutoMod Command"
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
                "AutoMod Help",
                "Available commands: `enable`, `disable`\n\nNote: AutoMod is now handled via Discord's native Server Settings.",
                module_ctx.embed_color,
            );
            let action_row = railway_common::ui::build_support_action_row();
            return ctx.reply_with_ui(embed, vec![action_row]).await;
        }

        let subcommand = args[0].to_lowercase();
        let reply_msg = match subcommand.as_str() {
            "enable" => self.handle_enable(guild_id, module_ctx).await?,
            "disable" => self.handle_disable(guild_id, module_ctx).await?,
            _ => "Unknown command. Use `enable` or `disable`.".to_string(),
        };

        let embed = railway_common::ui::build_stylish_embed(
            "AutoMod Command",
            &reply_msg,
            module_ctx.embed_color,
        );

        ctx.reply_with_ui(embed, vec![]).await?;

        Ok(())
    }
}

impl Default for AutomodCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
