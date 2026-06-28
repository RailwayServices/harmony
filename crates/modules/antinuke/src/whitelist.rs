use dashmap::DashMap;
use std::collections::HashSet;

#[derive(Debug)]
pub struct WhitelistStore {
    inner: DashMap<u64, HashSet<u64>>,
}

impl WhitelistStore {
    #[must_use]
    pub fn new() -> Self {
        Self { inner: DashMap::new() }
    }

    #[must_use]
    pub fn is_whitelisted(&self, guild_id: u64, user_id: u64) -> bool {
        self.inner.get(&guild_id).map(|s| s.contains(&user_id)).unwrap_or(false)
    }

    pub fn set(&self, guild_id: u64, user_ids: Vec<u64>) {
        self.inner.insert(guild_id, user_ids.into_iter().collect());
    }

    pub fn add(&self, guild_id: u64, user_id: u64) {
        self.inner.entry(guild_id).or_default().insert(user_id);
    }

    pub fn remove(&self, guild_id: u64, user_id: u64) {
        if let Some(mut s) = self.inner.get_mut(&guild_id) {
            s.remove(&user_id);
        }
    }

    pub fn remove_guild(&self, guild_id: u64) {
        self.inner.remove(&guild_id);
    }
}

impl Default for WhitelistStore {
    fn default() -> Self {
        Self::new()
    }
}
