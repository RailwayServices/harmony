pub mod actions;
pub mod filters;

use railway_common::error::RailwayError;
use railway_common::module::{Module, ModuleContext};

pub struct AutomodModule {}

impl AutomodModule {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AutomodModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutomodModule {
    fn name(&self) -> &'static str {
        "automod"
    }

    async fn handle_event(
        &self,
        _event: &railway_common::event::RailwayEvent,
        _ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        // In a real implementation we would convert the wrapper event to Discord events,
        // but for now we just return Ok
        Ok(())
    }
}
