use std::sync::Arc;
use twilight_model::gateway::event::Event;

#[derive(Debug, Clone)]
pub enum HarmonyEvent {
    Discord(Arc<Event>),
    Synthetic(String),
    SendToShard {
        guild_id: twilight_model::id::Id<twilight_model::id::marker::GuildMarker>,
        payload: String,
    },
}

impl From<Event> for HarmonyEvent {
    fn from(event: Event) -> Self {
        Self::Discord(Arc::new(event))
    }
}
