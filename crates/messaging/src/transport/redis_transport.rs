use crate::publisher::Publisher;
use crate::subscriber::Subscriber;
use futures::StreamExt;
use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};

pub struct RedisTransport {
    client: redis::Client,
    publish_channel: String,
    subscribe_channel: String,
    capacity: usize,
}

impl RedisTransport {
    pub fn new(
        redis_url: &str,
        publish_channel: &str,
        subscribe_channel: &str,
        capacity: usize,
    ) -> Result<Self, HarmonyError> {
        let client = redis::Client::open(redis_url).map_err(HarmonyError::Cache)?;
        Ok(Self {
            client,
            publish_channel: publish_channel.to_string(),
            subscribe_channel: subscribe_channel.to_string(),
            capacity,
        })
    }
}

impl Publisher for RedisTransport {
    async fn publish(&self, event: Arc<HarmonyEvent>) -> Result<(), HarmonyError> {
        let mut conn =
            self.client.get_multiplexed_async_connection().await.map_err(HarmonyError::Cache)?;
        let serialized = serde_json::to_string(&*event).map_err(|e| {
            HarmonyError::Internal(format!("Failed to serialize event for Redis: {}", e))
        })?;

        let _: () =
            conn.publish(&self.publish_channel, serialized).await.map_err(HarmonyError::Cache)?;
        Ok(())
    }
}

impl Subscriber for RedisTransport {
    async fn subscribe(&self) -> Result<broadcast::Receiver<Arc<HarmonyEvent>>, HarmonyError> {
        let (tx, rx) = broadcast::channel(self.capacity);
        let client = self.client.clone();
        let channel_name = self.subscribe_channel.clone();

        tokio::spawn(async move {
            loop {
                info!("Connecting to Redis Pub/Sub channel: {}", channel_name);
                match client.get_async_pubsub().await {
                    Ok(mut pubsub) => {
                        if let Err(e) = pubsub.subscribe(&channel_name).await {
                            error!("Failed to subscribe to Redis channel: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue;
                        }

                        let mut stream = pubsub.on_message();
                        while let Some(msg) = stream.next().await {
                            if let Ok(payload) = msg.get_payload::<String>() {
                                match serde_json::from_str::<HarmonyEvent>(&payload) {
                                    Ok(event) => {
                                        if tx.receiver_count() > 0 {
                                            if let Err(e) = tx.send(Arc::new(event)) {
                                                error!("Local broadcast failed: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to deserialize Redis event: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Redis PubSub connection error: {}", e);
                    }
                }
                error!("Redis Pub/Sub stream ended. Reconnecting in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        Ok(rx)
    }
}
