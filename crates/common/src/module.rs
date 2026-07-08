use crate::error::HarmonyError;
use crate::event::HarmonyEvent;
use sqlx::PgPool;
use std::sync::Arc;
use twilight_http::Client as DiscordClient;

pub struct ModuleContext {
    pub db: PgPool,
    pub cache: redis::aio::MultiplexedConnection,
    pub discord: Arc<DiscordClient>,
    pub embed_color: u32,
    pub event_tx: tokio::sync::mpsc::Sender<HarmonyEvent>,
}

pub trait Module: Send + Sync {
    fn name(&self) -> &'static str;

    fn handle_event(
        &self,
        event: &HarmonyEvent,
        ctx: &ModuleContext,
    ) -> impl std::future::Future<Output = Result<(), HarmonyError>> + Send;
}
