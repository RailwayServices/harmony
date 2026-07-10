use harmony_common::error::HarmonyError;
use twilight_model::application::interaction::Interaction;

pub struct InteractionContext {
    pub interaction: Interaction,
    pub guild_id: Option<twilight_model::id::Id<twilight_model::id::marker::GuildMarker>>,
    pub user_id: Option<twilight_model::id::Id<twilight_model::id::marker::UserMarker>>,
}

impl InteractionContext {
    pub fn new(interaction: Interaction) -> Self {
        let guild_id = interaction.guild_id;
        let user_id = interaction.author_id();
        Self { interaction, guild_id, user_id }
    }

    pub fn extract_command_name(&self) -> Result<&str, HarmonyError> {
        if let Some(
            twilight_model::application::interaction::InteractionData::ApplicationCommand(cmd),
        ) = &self.interaction.data
        {
            return Ok(&cmd.name);
        }
        Err(HarmonyError::Internal("Interaction has no command data".to_string()))
    }

    pub fn extract_custom_id(&self) -> Result<&str, HarmonyError> {
        if let Some(twilight_model::application::interaction::InteractionData::MessageComponent(
            data,
        )) = &self.interaction.data
        {
            return Ok(&data.custom_id);
        }
        Err(HarmonyError::Internal("Interaction is not a message component".to_string()))
    }

    pub fn is_component(&self) -> bool {
        matches!(
            &self.interaction.data,
            Some(twilight_model::application::interaction::InteractionData::MessageComponent(_))
        )
    }

    pub fn extract_string_option(&self, name: &str) -> Option<String> {
        if let Some(
            twilight_model::application::interaction::InteractionData::ApplicationCommand(cmd),
        ) = &self.interaction.data
        {
            for opt in &cmd.options {
                if opt.name == name {
                    if let twilight_model::application::interaction::application_command::CommandOptionValue::String(s) = &opt.value {
                        return Some(s.clone());
                    }
                }
            }
        }
        None
    }

    pub fn extract_integer_option(&self, name: &str) -> Option<i64> {
        if let Some(
            twilight_model::application::interaction::InteractionData::ApplicationCommand(cmd),
        ) = &self.interaction.data
        {
            for opt in &cmd.options {
                if opt.name == name {
                    if let twilight_model::application::interaction::application_command::CommandOptionValue::Integer(i) = &opt.value {
                        return Some(*i);
                    }
                }
            }
        }
        None
    }

    pub async fn defer_update(
        &self,
        module_ctx: &harmony_common::module::ModuleContext,
    ) -> Result<(), HarmonyError> {
        let interaction_client = module_ctx.discord.interaction(self.interaction.application_id);

        let response = twilight_model::http::interaction::InteractionResponse {
            kind: twilight_model::http::interaction::InteractionResponseType::DeferredUpdateMessage,
            data: None,
        };

        interaction_client
            .create_response(self.interaction.id, &self.interaction.token, &response)
            .await
            .map_err(|e| HarmonyError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn defer(
        &self,
        module_ctx: &harmony_common::module::ModuleContext,
    ) -> Result<(), HarmonyError> {
        let interaction_client = module_ctx.discord.interaction(self.interaction.application_id);

        let response = twilight_model::http::interaction::InteractionResponse {
            kind: twilight_model::http::interaction::InteractionResponseType::DeferredChannelMessageWithSource,
            data: None,
        };

        interaction_client
            .create_response(self.interaction.id, &self.interaction.token, &response)
            .await
            .map_err(|e| HarmonyError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn reply_str(
        &self,
        content: &str,
        module_ctx: &harmony_common::module::ModuleContext,
    ) -> Result<(), HarmonyError> {
        let interaction_client = module_ctx.discord.interaction(self.interaction.application_id);

        let data =
            twilight_util::builder::InteractionResponseDataBuilder::new().content(content).build();

        let response = twilight_model::http::interaction::InteractionResponse {
            kind:
                twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
            data: Some(data),
        };

        interaction_client
            .create_response(self.interaction.id, &self.interaction.token, &response)
            .await
            .map_err(|e| HarmonyError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn reply_embed(
        &self,
        embed: twilight_model::channel::message::Embed,
        module_ctx: &harmony_common::module::ModuleContext,
    ) -> Result<(), HarmonyError> {
        let interaction_client = module_ctx.discord.interaction(self.interaction.application_id);

        let data = twilight_util::builder::InteractionResponseDataBuilder::new()
            .embeds(vec![embed])
            .build();

        let response = twilight_model::http::interaction::InteractionResponse {
            kind:
                twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
            data: Some(data),
        };

        interaction_client
            .create_response(self.interaction.id, &self.interaction.token, &response)
            .await
            .map_err(|e| HarmonyError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn edit_embed(
        &self,
        embed: twilight_model::channel::message::Embed,
        module_ctx: &harmony_common::module::ModuleContext,
    ) -> Result<(), HarmonyError> {
        let interaction_client = module_ctx.discord.interaction(self.interaction.application_id);

        interaction_client
            .update_response(&self.interaction.token)
            .embeds(Some(&[embed]))
            .await
            .map_err(|e| HarmonyError::Internal(e.to_string()))?;

        Ok(())
    }
}
