use harmony_common::error::HarmonyError;
use redis::Client;
use redis::aio::MultiplexedConnection;

#[derive(Clone)]
pub struct CacheClient {
    pub client: Client,
    pub connection: MultiplexedConnection,
}

impl CacheClient {
    pub async fn connect(url: &str) -> Result<Self, HarmonyError> {
        let client = Client::open(url).map_err(HarmonyError::Cache)?;
        let connection =
            client.get_multiplexed_async_connection().await.map_err(HarmonyError::Cache)?;

        Ok(Self { client, connection })
    }
}
