use harmony_common::error::HarmonyError;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};

pub struct GuildStateCache {}

impl GuildStateCache {
    pub async fn get_state<T: for<'de> Deserialize<'de>>(
        conn: &mut MultiplexedConnection,
        key: &str,
    ) -> Result<Option<T>, HarmonyError> {
        let raw: Option<String> = conn.get(key).await.map_err(HarmonyError::Cache)?;

        match raw {
            Some(json) => {
                let parsed: T = serde_json::from_str(&json)
                    .map_err(|e| HarmonyError::Internal(e.to_string()))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn set_state<T: Serialize>(
        conn: &mut MultiplexedConnection,
        key: &str,
        state: &T,
        ttl_seconds: u64,
    ) -> Result<(), HarmonyError> {
        let json =
            serde_json::to_string(state).map_err(|e| HarmonyError::Internal(e.to_string()))?;
        let _: () = conn.set_ex(key, json, ttl_seconds).await.map_err(HarmonyError::Cache)?;
        Ok(())
    }

    pub async fn invalidate(
        conn: &mut MultiplexedConnection,
        key: &str,
    ) -> Result<(), HarmonyError> {
        let _: () = conn.del(key).await.map_err(HarmonyError::Cache)?;
        Ok(())
    }
}
