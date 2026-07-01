use crate::interaction::InteractionContext;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::InteractionData;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseType};
use twilight_util::builder::InteractionResponseDataBuilder;

pub mod config;

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
            ("enable", CommandOptionValue::SubCommand(options)) => {
                let filter = options
                    .iter()
                    .find(|o| o.name == "filter")
                    .and_then(|o| match &o.value {
                        CommandOptionValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "spam".to_string());
                self.handle_enable(guild_id.get() as i64, filter, module_ctx).await?
            }
            ("disable", CommandOptionValue::SubCommand(options)) => {
                let filter = options
                    .iter()
                    .find(|o| o.name == "filter")
                    .and_then(|o| match &o.value {
                        CommandOptionValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "spam".to_string());
                self.handle_disable(guild_id.get() as i64, filter, module_ctx).await?
            }
            ("punishment", CommandOptionValue::SubCommand(options)) => {
                let filter = options
                    .iter()
                    .find(|o| o.name == "filter")
                    .and_then(|o| match &o.value {
                        CommandOptionValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "spam".to_string());

                let action = options
                    .iter()
                    .find(|o| o.name == "action")
                    .and_then(|o| match &o.value {
                        CommandOptionValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "delete".to_string());

                self.handle_punishment(guild_id.get() as i64, filter, action, module_ctx).await?
            }
            ("settings", CommandOptionValue::SubCommand(_)) => {
                self.handle_settings(guild_id.get() as i64, module_ctx).await?
            }
            _ => "Unknown subcommand".to_string(),
        };

        let action_row = if subcommand.name.as_str() == "settings" {
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
            railway_common::ui::build_automod_settings_buttons(spam, antilink, ghostping)
        } else {
            railway_common::ui::build_support_action_row()
        };

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

        Ok(())
    }
}

impl Default for AutomodCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
