use crate::error::RailwayError;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub discord_token: String,
    pub database_url: String,
    pub redis_url: String,
    pub prefix: String,
    pub embed_color: u32,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, RailwayError> {
        dotenvy::dotenv().ok();
        let discord_token = env::var("DISCORD_TOKEN")
            .map_err(|_| RailwayError::Config("DISCORD_TOKEN not set".to_string()))?;
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| RailwayError::Config("DATABASE_URL not set".to_string()))?;
        let redis_url = env::var("REDIS_URL")
            .map_err(|_| RailwayError::Config("REDIS_URL not set".to_string()))?;

        let prefix =
            env::var("PREFIX").map_err(|_| RailwayError::Config("PREFIX not set".to_string()))?;

        let embed_color_str = env::var("EMBED_COLOR").unwrap_or_else(|_| "BEBEBE".to_string());
        let embed_color =
            u32::from_str_radix(embed_color_str.trim_start_matches('#'), 16).unwrap_or(0xBEBEBE);

        Ok(Self { discord_token, database_url, redis_url, prefix, embed_color })
    }
}
