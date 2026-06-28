use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;

pub trait Publisher: Send + Sync {
    fn publish(
        &self,
        event: RailwayEvent,
    ) -> impl std::future::Future<Output = Result<(), RailwayError>> + Send;
}
