use railway_common::error::RailwayError;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

pub struct RateLimiter {}

impl RateLimiter {
    pub async fn check_and_increment(
        conn: &mut MultiplexedConnection,
        key: &str,
        window_seconds: u64,
    ) -> Result<i64, RailwayError> {
        let count: i64 = conn.incr(key, 1).await.map_err(RailwayError::Cache)?;

        if count == 1 {
            let _: () =
                conn.expire(key, window_seconds as i64).await.map_err(RailwayError::Cache)?;
        }

        Ok(count)
    }
}
