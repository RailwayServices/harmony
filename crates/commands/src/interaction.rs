use railway_common::error::RailwayError;
use twilight_model::application::interaction::Interaction;

pub struct InteractionContext {
    pub interaction: Interaction,
}

impl InteractionContext {
    pub fn new(interaction: Interaction) -> Self {
        Self { interaction }
    }

    pub fn extract_command_name(&self) -> Result<&str, RailwayError> {
        if let Some(
            twilight_model::application::interaction::InteractionData::ApplicationCommand(cmd),
        ) = &self.interaction.data
        {
            return Ok(&cmd.name);
        }
        Err(RailwayError::Internal("Interaction has no command data".to_string()))
    }

    pub fn extract_custom_id(&self) -> Result<&str, RailwayError> {
        if let Some(twilight_model::application::interaction::InteractionData::MessageComponent(
            data,
        )) = &self.interaction.data
        {
            return Ok(&data.custom_id);
        }
        Err(RailwayError::Internal("Interaction is not a message component".to_string()))
    }

    pub fn is_component(&self) -> bool {
        matches!(
            &self.interaction.data,
            Some(twilight_model::application::interaction::InteractionData::MessageComponent(_))
        )
    }
}
