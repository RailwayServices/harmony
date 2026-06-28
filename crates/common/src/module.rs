use crate::error::RailwayError;
use crate::event::RailwayEvent;
use redis::Client as RedisClient;
use sqlx::PgPool;
use std::sync::Arc;
use twilight_http::Client as DiscordClient;

pub struct ModuleContext {
    pub db: PgPool,
    pub cache: RedisClient,
    pub discord: Arc<DiscordClient>,
}

pub trait Module: Send + Sync {
    fn name(&self) -> &'static str;

    fn handle_event(
        &self,
        event: &RailwayEvent,
        ctx: &ModuleContext,
    ) -> impl std::future::Future<Output = Result<(), RailwayError>> + Send;
}
