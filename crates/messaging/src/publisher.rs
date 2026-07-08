use harmony_common::error::HarmonyError;
use harmony_common::event::HarmonyEvent;
use std::sync::Arc;

pub trait Publisher: Send + Sync {
    fn publish(
        &self,
        event: Arc<HarmonyEvent>,
    ) -> impl std::future::Future<Output = Result<(), HarmonyError>> + Send;
}
