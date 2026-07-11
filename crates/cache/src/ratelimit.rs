use harmony_common::error::HarmonyError;
use redis::aio::MultiplexedConnection;

pub struct RateLimiter;

impl RateLimiter {
    pub async fn check_and_increment(
        conn: &mut MultiplexedConnection,
        key: &str,
        window_seconds: u64,
    ) -> Result<i64, HarmonyError> {
        let script = redis::Script::new(
            r#"
            local count = redis.call('INCR', KEYS[1])
            if count == 1 then
                redis.call('EXPIRE', KEYS[1], ARGV[1])
            end
            return count
            "#,
        );

        let count: i64 = script
            .key(key)
            .arg(window_seconds as i64)
            .invoke_async(conn)
            .await
            .map_err(HarmonyError::Cache)?;

        Ok(count)
    }
}
