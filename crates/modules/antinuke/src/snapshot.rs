use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct PermOverwrite {
    pub id: u64,
    pub kind: u8,
    pub allow: u64,
    pub deny: u64,
}

#[derive(Debug, Clone)]
pub struct ChannelSnap {
    pub id: u64,
    pub name: String,
    pub kind: u8,
    pub position: i32,
    pub topic: Option<String>,
    pub nsfw: bool,
    pub rate_limit_per_user: u16,
    pub parent_id: Option<u64>,
    pub overwrites: Vec<PermOverwrite>,
}

#[derive(Debug, Clone)]
pub struct RoleSnap {
    pub id: u64,
    pub name: String,
    pub color: u32,
    pub permissions: u64,
    pub position: i64,
    pub hoist: bool,
    pub mentionable: bool,
}

#[derive(Debug, Clone)]
pub struct GuildSnapshot {
    pub guild_id: u64,
    pub name: String,
    pub channels: Vec<ChannelSnap>,
    pub roles: Vec<RoleSnap>,
}

#[derive(Debug)]
pub struct SnapshotStore {
    inner: DashMap<u64, GuildSnapshot>,
}

impl SnapshotStore {
    #[must_use]
    pub fn new() -> Self {
        Self { inner: DashMap::new() }
    }

    pub fn set(&self, snap: GuildSnapshot) {
        self.inner.insert(snap.guild_id, snap);
    }

    #[must_use]
    pub fn get(&self, guild_id: u64) -> Option<GuildSnapshot> {
        self.inner.get(&guild_id).map(|s| s.clone())
    }

    pub fn upsert_channel(&self, guild_id: u64, channel: ChannelSnap) {
        if let Some(mut snap) = self.inner.get_mut(&guild_id) {
            if let Some(existing) = snap.channels.iter_mut().find(|c| c.id == channel.id) {
                *existing = channel;
            } else {
                snap.channels.push(channel);
            }
        }
    }

    pub fn remove_channel(&self, guild_id: u64, channel_id: u64) {
        if let Some(mut snap) = self.inner.get_mut(&guild_id) {
            snap.channels.retain(|c| c.id != channel_id);
        }
    }

    pub fn upsert_role(&self, guild_id: u64, role: RoleSnap) {
        if let Some(mut snap) = self.inner.get_mut(&guild_id) {
            if let Some(existing) = snap.roles.iter_mut().find(|r| r.id == role.id) {
                *existing = role;
            } else {
                snap.roles.push(role);
            }
        }
    }

    pub fn remove_role(&self, guild_id: u64, role_id: u64) {
        if let Some(mut snap) = self.inner.get_mut(&guild_id) {
            snap.roles.retain(|r| r.id != role_id);
        }
    }

    pub fn remove_guild(&self, guild_id: u64) {
        self.inner.remove(&guild_id);
    }
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}
