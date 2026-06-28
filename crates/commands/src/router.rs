use crate::handlers::antinuke_commands::AntinukeCommandHandler;
use crate::interaction::InteractionContext;
use crate::prefix::PrefixRouter;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;

pub struct CommandRouter {
    antinuke_handler: AntinukeCommandHandler,
    prefix_router: PrefixRouter,
}

impl CommandRouter {
    pub fn new(prefix: String) -> Self {
        Self {
            antinuke_handler: AntinukeCommandHandler::new(),
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
        let name = interaction_ctx.extract_command_name()?;

        match name {
            "antinuke" => self.antinuke_handler.handle(interaction_ctx, module_ctx).await,
            _ => Err(RailwayError::Internal(format!("Unknown command: {}", name))),
        }
    }
}
