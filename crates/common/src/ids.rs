use twilight_model::id::marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker};
use twilight_model::id::Id;

pub type GuildId = Id<GuildMarker>;
pub type UserId = Id<UserMarker>;
pub type ChannelId = Id<ChannelMarker>;
pub type RoleId = Id<RoleMarker>;
