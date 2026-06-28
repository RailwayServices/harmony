use railway_common::error::RailwayError;
use redis::aio::MultiplexedConnection;
use redis::Client;

#[derive(Clone)]
pub struct CacheClient {
    pub connection: MultiplexedConnection,
}

impl CacheClient {
    pub async fn connect(url: &str) -> Result<Self, RailwayError> {
        let client = Client::open(url).map_err(RailwayError::Cache)?;
        let connection =
            client.get_multiplexed_tokio_connection().await.map_err(RailwayError::Cache)?;

        Ok(Self { connection })
    }
}
