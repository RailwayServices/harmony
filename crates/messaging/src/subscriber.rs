use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use std::sync::Arc;

pub trait Subscriber: Send + Sync + 'static {
    fn subscribe(
        &self,
    ) -> impl std::future::Future<
        Output = Result<tokio::sync::broadcast::Receiver<Arc<HarmonyEvent>>, HarmonyError>,
    > + Send;
}
