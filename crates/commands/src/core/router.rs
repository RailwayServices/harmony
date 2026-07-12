use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixRouter;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;

pub struct CommandRouter {
    prefix_router: PrefixRouter,
}

impl CommandRouter {
    pub fn new(prefix: String) -> Self {
        Self { prefix_router: PrefixRouter::new(prefix) }
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

        match name {
            "play" => crate::music::play::handle(interaction_ctx, module_ctx).await,
            "stop" => crate::music::control::handle_stop(interaction_ctx, module_ctx).await,
            "skip" => crate::music::control::handle_skip(interaction_ctx, module_ctx).await,
            "pause" => crate::music::control::handle_pause(interaction_ctx, module_ctx).await,
            "resume" => crate::music::control::handle_resume(interaction_ctx, module_ctx).await,
            "volume" => crate::music::control::handle_volume(interaction_ctx, module_ctx).await,
            "filter" => crate::music::filter::handle_filter(interaction_ctx, module_ctx).await,
            "queue" => crate::music::queue::handle_queue(interaction_ctx, module_ctx).await,
            other => {
                tracing::debug!("Received unknown command '{}', ignoring.", other);
                Ok(())
            }
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
