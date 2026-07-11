use harmony_common::error::HarmonyError;
use redis::aio::MultiplexedConnection;

pub struct DistributedLock;

impl DistributedLock {
    pub async fn acquire(
        conn: &mut MultiplexedConnection,
        key: &str,
        ttl_millis: u64,
    ) -> Result<Option<String>, HarmonyError> {
        let token = uuid::Uuid::new_v4().to_string();

        let result: Option<String> = redis::cmd("SET")
            .arg(key)
            .arg(&token)
            .arg("NX")
            .arg("PX")
            .arg(ttl_millis)
            .query_async(conn)
            .await
            .map_err(HarmonyError::Cache)?;

        Ok(result.map(|_| token))
    }

    pub async fn release(
        conn: &mut MultiplexedConnection,
        key: &str,
        token: &str,
    ) -> Result<bool, HarmonyError> {
        let script = redis::Script::new(
            r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("DEL", KEYS[1])
            else
                return 0
            end
            "#,
        );

        let result: i64 =
            script.key(key).arg(token).invoke_async(conn).await.map_err(HarmonyError::Cache)?;

        Ok(result == 1)
    }
}
