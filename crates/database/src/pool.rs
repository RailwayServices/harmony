use harmony_common::error::HarmonyError;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn connect(url: &str, pool_size: u32) -> Result<Self, HarmonyError> {
        let pool = PgPoolOptions::new()
            .max_connections(pool_size)
            .min_connections(pool_size.min(5))
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .test_before_acquire(false)
            .connect(url)
            .await
            .map_err(HarmonyError::Database)?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
    }
}
