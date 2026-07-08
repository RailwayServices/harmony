use crate::publisher::Publisher;
use crate::subscriber::Subscriber;
use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct LocalTransport {
    sender: broadcast::Sender<Arc<HarmonyEvent>>,
}

impl LocalTransport {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

impl Publisher for LocalTransport {
    async fn publish(&self, event: Arc<HarmonyEvent>) -> Result<(), HarmonyError> {
        if self.sender.receiver_count() > 0 {
            self.sender.send(event).map_err(|e| {
                HarmonyError::Internal(format!("Failed to publish local event: {}", e))
            })?;
        }
        Ok(())
    }
}

impl Subscriber for LocalTransport {
    async fn subscribe(&self) -> Result<broadcast::Receiver<Arc<HarmonyEvent>>, HarmonyError> {
        Ok(self.sender.subscribe())
    }
}
