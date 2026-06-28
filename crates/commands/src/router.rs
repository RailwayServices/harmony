use crate::handlers::antinuke_commands::AntinukeCommandHandler;
use crate::interaction::InteractionContext;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;

pub struct CommandRouter {
    antinuke_handler: AntinukeCommandHandler,
}

impl CommandRouter {
    pub fn new() -> Self {
        Self { antinuke_handler: AntinukeCommandHandler::new() }
    }

    pub async fn handle_event(
        &self,
        event: &railway_common::event::RailwayEvent,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        if let railway_common::event::RailwayEvent::Discord(box_event) = event {
            if let twilight_model::gateway::event::Event::InteractionCreate(interaction) =
                &**box_event
            {
                let interaction_ctx = InteractionContext::new(interaction.0.clone());
                return self.route(&interaction_ctx, module_ctx).await;
            }
        }
        Ok(())
    }

    pub async fn route(
        &self,
        interaction_ctx: &InteractionContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        let name = interaction_ctx.extract_command_name()?;

        match name {
            "antinuke" => self.antinuke_handler.handle(interaction_ctx, module_ctx).await,
            _ => Err(RailwayError::Internal(format!("Unknown command: {}", name))),
        }
    }
}

impl Default for CommandRouter {
    fn default() -> Self {
        Self::new()
    }
}
