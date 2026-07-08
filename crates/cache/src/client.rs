use harmony_common::error::HarmonyError;
use redis::aio::MultiplexedConnection;
use redis::Client;

#[derive(Clone)]
pub struct CacheClient {
    pub connection: MultiplexedConnection,
}

impl CacheClient {
    pub async fn connect(url: &str) -> Result<Self, HarmonyError> {
        let client = Client::open(url).map_err(HarmonyError::Cache)?;
        let connection =
            client.get_multiplexed_tokio_connection().await.map_err(HarmonyError::Cache)?;

        Ok(Self { connection })
    }
}
