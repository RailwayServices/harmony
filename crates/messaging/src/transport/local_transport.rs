use crate::publisher::Publisher;
use crate::subscriber::Subscriber;
use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use tokio::sync::broadcast;

pub struct LocalTransport {
    sender: broadcast::Sender<RailwayEvent>,
}

impl LocalTransport {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

impl Publisher for LocalTransport {
    async fn publish(&self, event: RailwayEvent) -> Result<(), RailwayError> {
        if self.sender.receiver_count() > 0 {
            self.sender.send(event).map_err(|e| {
                RailwayError::Internal(format!("Failed to publish local event: {}", e))
            })?;
        }
        Ok(())
    }
}

impl Subscriber for LocalTransport {
    async fn subscribe(&self) -> Result<broadcast::Receiver<RailwayEvent>, RailwayError> {
        Ok(self.sender.subscribe())
    }
}
