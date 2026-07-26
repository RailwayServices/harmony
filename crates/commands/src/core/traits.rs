use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use twilight_model::application::command::Command;
use twilight_model::application::interaction::application_command::CommandData;

#[async_trait::async_trait]
pub trait AppCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn register(&self) -> Command;
    async fn handle(
        &self,
        interaction_ctx: &InteractionContext,
        data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError>;
}

#[async_trait::async_trait]
pub trait PrefixCommand: Send + Sync {
    fn aliases(&self) -> Vec<&'static str>;
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError>;
}
