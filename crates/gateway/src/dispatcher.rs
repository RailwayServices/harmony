use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use harmony_messaging::publisher::Publisher;
use std::sync::Arc;

pub struct EventDispatcher<P: Publisher> {
    publisher: Arc<P>,
}

impl<P: Publisher> EventDispatcher<P> {
    pub fn new(publisher: Arc<P>) -> Self {
        Self { publisher }
    }

    pub async fn dispatch(&self, event: HarmonyEvent) -> Result<(), HarmonyError> {
        self.publisher.publish(Arc::new(event)).await
    }
}
