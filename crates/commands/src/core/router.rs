use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixRouter;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;

use crate::core::traits::AppCommand;
use std::collections::HashMap;

pub struct CommandRouter {
    prefix_router: PrefixRouter,
    slash_commands: HashMap<&'static str, Box<dyn AppCommand>>,
}

impl CommandRouter {
    pub fn new(prefix: String) -> Self {
        let mut slash_commands: HashMap<&'static str, Box<dyn AppCommand>> = HashMap::new();

        let play = Box::new(crate::music::play::PlayAppCommand);
        slash_commands.insert(play.name(), play);

        let stop = Box::new(crate::music::control::StopAppCommand);
        slash_commands.insert(stop.name(), stop);

        let skip = Box::new(crate::music::control::SkipAppCommand);
        slash_commands.insert(skip.name(), skip);

        let pause = Box::new(crate::music::control::PauseAppCommand);
        slash_commands.insert(pause.name(), pause);

        let resume = Box::new(crate::music::control::ResumeAppCommand);
        slash_commands.insert(resume.name(), resume);

        let volume = Box::new(crate::music::control::VolumeAppCommand);
        slash_commands.insert(volume.name(), volume);

        let filter = Box::new(crate::music::filter::FilterAppCommand);
        slash_commands.insert(filter.name(), filter);

        let queue = Box::new(crate::music::queue::QueueAppCommand);
        slash_commands.insert(queue.name(), queue);

        Self { prefix_router: PrefixRouter::new(prefix), slash_commands }
    }

    pub fn get_commands(&self) -> Vec<twilight_model::application::command::Command> {
        self.slash_commands.values().map(|cmd| cmd.register()).collect()
    }

    pub async fn handle_event(
        &self,
        event: &harmony_common::event::HarmonyEvent,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        if let harmony_common::event::HarmonyEvent::Discord(arc_event) = event {
            if let harmony_common::event::SerializableEvent::InteractionCreate(interaction) =
                arc_event.as_ref()
            {
                let interaction_ctx = InteractionContext::new(interaction.0.clone());
                return self.route(&interaction_ctx, module_ctx).await;
            }
            if let harmony_common::event::SerializableEvent::MessageCreate(msg) = arc_event.as_ref()
            {
                return self.prefix_router.handle_message(&msg.0, module_ctx).await;
            }
        }
        Ok(())
    }

    pub async fn route(
        &self,
        interaction_ctx: &InteractionContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        if interaction_ctx.is_component() {
            let custom_id = interaction_ctx.extract_custom_id()?;
            return self.handle_interaction(interaction_ctx, custom_id, module_ctx).await;
        }

        let name = interaction_ctx.extract_command_name()?;

        let data = match &interaction_ctx.interaction.data {
            Some(
                twilight_model::application::interaction::InteractionData::ApplicationCommand(data),
            ) => data,
            _ => return Err(HarmonyError::Internal("Missing command data".to_string())),
        };

        if let Some(cmd) = self.slash_commands.get(name) {
            cmd.handle(interaction_ctx, data.as_ref(), module_ctx).await
        } else {
            tracing::debug!("Received unknown command '{}', ignoring.", name);
            Ok(())
        }
    }

    pub async fn handle_interaction(
        &self,
        interaction_ctx: &InteractionContext,
        custom_id: &str,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        // music_stop and music_skip are handled natively by the Gateway
        if custom_id == "music_stop" || custom_id == "music_skip" {
            return Ok(());
        }

        // Defer update so any unhandled button doesn't show "Interaction Failed"
        let _ = interaction_ctx.defer_update(module_ctx).await;
        Ok(())
    }
}
