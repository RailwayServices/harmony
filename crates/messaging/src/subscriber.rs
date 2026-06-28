use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;

pub trait Subscriber: Send + Sync + 'static {
    fn subscribe(
        &self,
    ) -> impl std::future::Future<
        Output = Result<tokio::sync::broadcast::Receiver<RailwayEvent>, RailwayError>,
    > + Send;
}
