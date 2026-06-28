use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_messaging::publisher::Publisher;
use std::sync::Arc;

pub struct EventDispatcher<P: Publisher> {
    publisher: Arc<P>,
}

impl<P: Publisher> EventDispatcher<P> {
    pub fn new(publisher: Arc<P>) -> Self {
        Self { publisher }
    }

    pub async fn dispatch(&self, event: RailwayEvent) -> Result<(), RailwayError> {
        self.publisher.publish(event).await
    }
}
