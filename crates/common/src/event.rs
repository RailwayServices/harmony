use serde::{Deserialize, Serialize};
use std::sync::Arc;
use twilight_model::gateway::event::Event;
use twilight_model::gateway::payload::incoming::{
    GuildCreate, GuildDelete, InteractionCreate, MessageCreate, Ready, VoiceServerUpdate,
    VoiceStateUpdate,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializableEvent {
    MessageCreate(Box<MessageCreate>),
    InteractionCreate(Box<InteractionCreate>),
    VoiceStateUpdate(Box<VoiceStateUpdate>),
    VoiceServerUpdate(Box<VoiceServerUpdate>),
    GuildCreate(Box<GuildCreate>),
    GuildDelete(Box<GuildDelete>),
    Ready(Box<Ready>),
    Other(String),
}

impl From<Event> for SerializableEvent {
    fn from(event: Event) -> Self {
        match event {
            Event::MessageCreate(e) => Self::MessageCreate(e),
            Event::InteractionCreate(e) => Self::InteractionCreate(e),
            Event::VoiceStateUpdate(e) => Self::VoiceStateUpdate(e),
            Event::VoiceServerUpdate(e) => Self::VoiceServerUpdate(Box::new(e)),
            Event::GuildCreate(e) => Self::GuildCreate(e),
            Event::GuildDelete(e) => Self::GuildDelete(Box::new(e)),
            Event::Ready(e) => Self::Ready(Box::new(e)),
            _ => Self::Other(format!("{:?}", event.kind())),
        }
    }
}

impl SerializableEvent {
    pub fn kind_name(&self) -> &str {
        match self {
            Self::MessageCreate(_) => "MessageCreate",
            Self::InteractionCreate(_) => "InteractionCreate",
            Self::VoiceStateUpdate(_) => "VoiceStateUpdate",
            Self::VoiceServerUpdate(_) => "VoiceServerUpdate",
            Self::GuildCreate(_) => "GuildCreate",
            Self::GuildDelete(_) => "GuildDelete",
            Self::Ready(_) => "Ready",
            Self::Other(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HarmonyEvent {
    Discord(Arc<SerializableEvent>),
    Synthetic(String),
    SendToShard {
        guild_id: twilight_model::id::Id<twilight_model::id::marker::GuildMarker>,
        payload: String,
    },
}

impl From<Event> for HarmonyEvent {
    fn from(event: Event) -> Self {
        Self::Discord(Arc::new(SerializableEvent::from(event)))
    }
}
