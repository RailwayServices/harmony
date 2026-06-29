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
    channels: DashMap<(u64, u64), ChannelSnap>,
    roles: DashMap<(u64, u64), RoleSnap>,
    members: DashMap<(u64, u64), Vec<u64>>,
}

impl SnapshotStore {
    #[must_use]
    pub fn new() -> Self {
        Self { channels: DashMap::new(), roles: DashMap::new(), members: DashMap::new() }
    }

    pub fn set(&self, snap: GuildSnapshot) {
        let gid = snap.guild_id;
        self.channels.retain(|k, _| k.0 != gid);
        self.roles.retain(|k, _| k.0 != gid);
        for ch in snap.channels {
            self.channels.insert((gid, ch.id), ch);
        }
        for r in snap.roles {
            self.roles.insert((gid, r.id), r);
        }
    }

    #[must_use]
    pub fn get_channel(&self, guild_id: u64, channel_id: u64) -> Option<ChannelSnap> {
        self.channels.get(&(guild_id, channel_id)).map(|r| r.clone())
    }

    pub fn upsert_channel(&self, guild_id: u64, channel: ChannelSnap) {
        self.channels.insert((guild_id, channel.id), channel);
    }

    pub fn remove_channel(&self, guild_id: u64, channel_id: u64) {
        self.channels.remove(&(guild_id, channel_id));
    }

    #[must_use]
    pub fn get_role(&self, guild_id: u64, role_id: u64) -> Option<RoleSnap> {
        self.roles.get(&(guild_id, role_id)).map(|r| r.clone())
    }

    #[must_use]
    pub fn get_role_perms(&self, guild_id: u64, role_id: u64) -> u64 {
        self.roles.get(&(guild_id, role_id)).map(|r| r.permissions).unwrap_or(0)
    }

    pub fn upsert_role(&self, guild_id: u64, role: RoleSnap) {
        self.roles.insert((guild_id, role.id), role);
    }

    pub fn remove_role(&self, guild_id: u64, role_id: u64) {
        self.roles.remove(&(guild_id, role_id));
    }

    pub fn set_member_roles(&self, guild_id: u64, user_id: u64, roles: Vec<u64>) {
        self.members.insert((guild_id, user_id), roles);
    }

    #[must_use]
    pub fn get_member_roles(&self, guild_id: u64, user_id: u64) -> Option<Vec<u64>> {
        self.members.get(&(guild_id, user_id)).map(|r| r.clone())
    }

    pub fn remove_guild(&self, guild_id: u64) {
        self.channels.retain(|k, _| k.0 != guild_id);
        self.roles.retain(|k, _| k.0 != guild_id);
        self.members.retain(|k, _| k.0 != guild_id);
    }
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}
