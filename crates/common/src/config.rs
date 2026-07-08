use crate::error::HarmonyError;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub discord_token: String,
    pub database_url: String,
    pub redis_url: String,
    pub prefix: String,
    pub embed_color: u32,
    pub db_pool_size: u32,
    pub event_bus_capacity: usize,
    pub max_event_tasks: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, HarmonyError> {
        dotenvy::dotenv().ok();

        let discord_token = env::var("DISCORD_TOKEN")
            .map_err(|_| HarmonyError::Config("DISCORD_TOKEN not set".to_string()))?;
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| HarmonyError::Config("DATABASE_URL not set".to_string()))?;
        let redis_url = env::var("REDIS_URL")
            .map_err(|_| HarmonyError::Config("REDIS_URL not set".to_string()))?;
        let prefix =
            env::var("PREFIX").map_err(|_| HarmonyError::Config("PREFIX not set".to_string()))?;

        let embed_color = u32::from_str_radix(
            env::var("EMBED_COLOR")
                .unwrap_or_else(|_| "BEBEBE".to_string())
                .trim_start_matches('#'),
            16,
        )
        .unwrap_or(0xBEBEBE);

        let db_pool_size =
            env::var("DATABASE_POOL_SIZE").ok().and_then(|v| v.parse().ok()).unwrap_or(20u32);

        let event_bus_capacity =
            env::var("EVENT_BUS_CAPACITY").ok().and_then(|v| v.parse().ok()).unwrap_or(16_384usize);

        let max_event_tasks =
            env::var("MAX_EVENT_TASKS").ok().and_then(|v| v.parse().ok()).unwrap_or(512usize);

        Ok(Self {
            discord_token,
            database_url,
            redis_url,
            prefix,
            embed_color,
            db_pool_size,
            event_bus_capacity,
            max_event_tasks,
        })
    }
}
