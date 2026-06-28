use railway_common::error::RailwayError;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

pub struct GuildStateCache {}

impl GuildStateCache {
    pub async fn get_state<T: for<'de> Deserialize<'de>>(
        conn: &mut MultiplexedConnection,
        key: &str,
    ) -> Result<Option<T>, RailwayError> {
        let raw: Option<String> = conn.get(key).await.map_err(RailwayError::Cache)?;

        match raw {
            Some(json) => {
                let parsed: T = serde_json::from_str(&json)
                    .map_err(|e| RailwayError::Internal(e.to_string()))?;
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
    ) -> Result<(), RailwayError> {
        let json =
            serde_json::to_string(state).map_err(|e| RailwayError::Internal(e.to_string()))?;
        let _: () = conn.set_ex(key, json, ttl_seconds).await.map_err(RailwayError::Cache)?;
        Ok(())
    }

    pub async fn invalidate(
        conn: &mut MultiplexedConnection,
        key: &str,
    ) -> Result<(), RailwayError> {
        let _: () = conn.del(key).await.map_err(RailwayError::Cache)?;
        Ok(())
    }
}
