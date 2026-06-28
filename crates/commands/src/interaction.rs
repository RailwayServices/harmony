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
}
