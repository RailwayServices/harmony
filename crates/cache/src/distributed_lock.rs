use harmony_common::error::HarmonyError;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

pub struct DistributedLock {}

impl DistributedLock {
    pub async fn acquire(
        conn: &mut MultiplexedConnection,
        key: &str,
        ttl_millis: u64,
    ) -> Result<bool, HarmonyError> {
        let result: Option<String> = redis::cmd("SET")
            .arg(key)
            .arg("1")
            .arg("NX")
            .arg("PX")
            .arg(ttl_millis)
            .query_async(conn)
            .await
            .map_err(HarmonyError::Cache)?;

        Ok(result.is_some())
    }

    pub async fn release(conn: &mut MultiplexedConnection, key: &str) -> Result<(), HarmonyError> {
        let _: () = conn.del(key).await.map_err(HarmonyError::Cache)?;
        Ok(())
    }
}
