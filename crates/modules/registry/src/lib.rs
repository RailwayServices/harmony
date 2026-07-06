use railway_antinuke::AntinukeModule;
use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_common::module::{Module, ModuleContext};

use railway_database::pool::Database;
use std::sync::Arc;
use twilight_http::Client as HttpClient;

pub struct ModuleRegistry {
    antinuke: AntinukeModule,
}

impl ModuleRegistry {
    pub fn new(http: Arc<HttpClient>, db: Database) -> Self {
        Self { antinuke: AntinukeModule::new(http, db) }
    }

    pub async fn handle_event(
        &self,
        event: &RailwayEvent,
        ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        // Sequentially route the event to each registered module
        self.antinuke.handle_event(event, ctx).await?;

        Ok(())
    }
}
