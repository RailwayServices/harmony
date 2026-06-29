use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;

use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    content::{scan, ContentScanInput},
    scorer::{compute_score, has_dangerous_perm_grant, resolve_punishment},
    snapshot::{ChannelSnap, GuildSnapshot, RoleSnap, SnapshotStore},
    types::{ActionType, ContentMatch, GuildConfig, Punishment, ThreatResult},
    whitelist::WhitelistStore,
};

struct GuildState {
    config: ArcSwap<GuildConfig>,
    whitelist: HashSet<u64>,
}

pub struct AntiNukeEngine {
    guilds: DashMap<u64, GuildState>,
    snapshots: SnapshotStore,
    whitelist_store: WhitelistStore,
    active_punishments: DashMap<(u64, u64), ()>,
}

impl AntiNukeEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            guilds: DashMap::new(),
            snapshots: SnapshotStore::new(),
            whitelist_store: WhitelistStore::new(),
            active_punishments: DashMap::new(),
        }
    }

    pub fn configure(&self, guild_id: u64, config: GuildConfig, whitelist_ids: Vec<u64>) {
        let whitelist: HashSet<u64> = whitelist_ids.iter().copied().collect();
        self.whitelist_store.set(guild_id, whitelist_ids);

        if let Some(state) = self.guilds.get(&guild_id) {
            state.config.store(Arc::new(config));
        } else {
            let state = GuildState { config: ArcSwap::from_pointee(config), whitelist };
            self.guilds.insert(guild_id, state);
        }
    }

    pub fn remove_guild(&self, guild_id: u64) {
        self.guilds.remove(&guild_id);
        self.snapshots.remove_guild(guild_id);
        self.whitelist_store.remove_guild(guild_id);
        self.active_punishments.retain(|k, _| k.0 != guild_id);
    }

    pub async fn clear_user(
        &self,
        guild_id: u64,
        user_id: u64,
        redis: &mut redis::aio::MultiplexedConnection,
    ) {
        self.active_punishments.remove(&(guild_id, user_id));

        let mut pipe = redis::pipe();
        for action in 0..10 {
            let key = format!("railway:antinuke:windows:{}:{}:{}", guild_id, user_id, action);
            pipe.del(key);
        }
        let _ = pipe.query_async::<()>(redis).await;
    }

    pub fn try_claim_punishment(&self, guild_id: u64, user_id: u64) -> bool {
        match self.active_punishments.entry((guild_id, user_id)) {
            dashmap::mapref::entry::Entry::Occupied(_) => false,
            dashmap::mapref::entry::Entry::Vacant(v) => {
                v.insert(());
                true
            }
        }
    }

    pub async fn process_event(
        &self,
        guild_id: u64,
        user_id: u64,
        action: ActionType,
        extra_data: Option<&(u64, u64)>,
        redis_conn: &mut redis::aio::MultiplexedConnection,
    ) -> Option<ThreatResult> {
        let guild = self.guilds.get(&guild_id)?;
        let config = guild.config.load();

        if !config.enabled {
            return None;
        }

        if guild.whitelist.contains(&user_id)
            || self.whitelist_store.is_whitelisted(guild_id, user_id)
        {
            return None;
        }

        let module_cfg = match config.modules.get(&(action as u8)) {
            Some(m) if m.enabled => m,
            _ => return None,
        };

        if action == ActionType::RoleUpdate {
            if let Some(&(old_perms, new_perms)) = extra_data {
                if !has_dangerous_perm_grant(old_perms, new_perms) {
                    return None;
                }
            } else {
                return None;
            }
        }

        let now_ms =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        if action.is_instant() || module_cfg.window_secs == 0 {
            let score = compute_score(action, 1);
            let (punishment_out, reason) =
                resolve_punishment(module_cfg.punishment, action, 1, score);
            return Some(ThreatResult {
                score,
                triggered: !module_cfg.log_only,
                punishment: punishment_out,
                reason,
                action,
                should_restore: action.requires_restore(),
                count_in_window: 1,
            });
        }

        let key = format!("railway:antinuke:windows:{}:{}:{}", guild_id, user_id, action as u8);
        let window_ms = (module_cfg.window_secs as u64) * 1000;
        let cutoff_ms = now_ms.saturating_sub(window_ms);

        let mut pipe = redis::pipe();
        pipe.cmd("ZREMRANGEBYSCORE").arg(&key).arg(0).arg(cutoff_ms).ignore();
        pipe.cmd("ZADD").arg(&key).arg(now_ms).arg(now_ms).ignore();
        pipe.cmd("ZCARD").arg(&key);
        pipe.cmd("EXPIRE").arg(&key).arg(module_cfg.window_secs).ignore();

        let result: redis::RedisResult<(usize,)> = pipe.query_async(redis_conn).await;

        let count = match result {
            Ok((c,)) => c,
            Err(_) => 1,
        };

        let mut score = compute_score(action, count);
        if module_cfg.threshold == 0 {
            score = 100;
        }
        let triggered = count >= module_cfg.threshold as usize;

        if !triggered {
            if action.requires_restore() {
                return Some(ThreatResult {
                    score,
                    triggered: false,
                    punishment: Punishment::None,
                    reason: String::new(),
                    action,
                    should_restore: true,
                    count_in_window: count as u32,
                });
            }
            return None;
        }

        let (punishment_out, reason) =
            resolve_punishment(module_cfg.punishment, action, count as u32, score);

        Some(ThreatResult {
            score,
            triggered: !module_cfg.log_only,
            punishment: punishment_out,
            reason,
            action,
            should_restore: action.requires_restore(),
            count_in_window: count as u32,
        })
    }

    pub fn scan_content(
        &self,
        guild_id: u64,
        user_id: u64,
        content: &str,
        author_roles: &[u64],
    ) -> Vec<ContentMatch> {
        let guild = match self.guilds.get(&guild_id) {
            Some(g) => g,
            None => return Vec::new(),
        };

        let config = guild.config.load();
        if !config.enabled {
            return Vec::new();
        }

        if guild.whitelist.contains(&user_id)
            || self.whitelist_store.is_whitelisted(guild_id, user_id)
        {
            return Vec::new();
        }

        let input = ContentScanInput { content, author_roles };
        scan(&input, &config.modules)
    }

    pub async fn process_content_match(
        &self,
        guild_id: u64,
        user_id: u64,
        module: ActionType,
        redis_conn: &mut redis::aio::MultiplexedConnection,
    ) -> Option<ThreatResult> {
        self.process_event(guild_id, user_id, module, None, redis_conn).await
    }

    pub fn set_snapshot(&self, snap: GuildSnapshot) {
        self.snapshots.set(snap);
    }

    #[must_use]
    pub fn get_channel_snap(&self, guild_id: u64, channel_id: u64) -> Option<ChannelSnap> {
        self.snapshots.get_channel(guild_id, channel_id)
    }

    #[must_use]
    pub fn get_role_snap(&self, guild_id: u64, role_id: u64) -> Option<RoleSnap> {
        self.snapshots.get_role(guild_id, role_id)
    }

    #[must_use]
    pub fn get_role_perms(&self, guild_id: u64, role_id: u64) -> u64 {
        self.snapshots.get_role_perms(guild_id, role_id)
    }

    pub fn upsert_channel_snap(&self, guild_id: u64, ch: ChannelSnap) {
        self.snapshots.upsert_channel(guild_id, ch);
    }

    pub fn remove_channel_snap(&self, guild_id: u64, channel_id: u64) {
        self.snapshots.remove_channel(guild_id, channel_id);
    }

    pub fn upsert_role_snap(&self, guild_id: u64, role: RoleSnap) {
        self.snapshots.upsert_role(guild_id, role);
    }

    pub fn remove_role_snap(&self, guild_id: u64, role_id: u64) {
        self.snapshots.remove_role(guild_id, role_id);
    }

    pub fn set_member_roles(&self, guild_id: u64, user_id: u64, roles: Vec<u64>) {
        self.snapshots.set_member_roles(guild_id, user_id, roles);
    }

    #[must_use]
    pub fn get_member_roles(&self, guild_id: u64, user_id: u64) -> Option<Vec<u64>> {
        self.snapshots.get_member_roles(guild_id, user_id)
    }

    #[must_use]
    pub fn get_log_channel(&self, guild_id: u64) -> Option<u64> {
        self.guilds.get(&guild_id).and_then(|g| g.config.load().log_channel_id)
    }

    pub fn whitelist_add(&self, guild_id: u64, user_id: u64) {
        self.whitelist_store.add(guild_id, user_id);
        if let Some(mut g) = self.guilds.get_mut(&guild_id) {
            g.whitelist.insert(user_id);
        }
    }

    pub fn whitelist_remove(&self, guild_id: u64, user_id: u64) {
        self.whitelist_store.remove(guild_id, user_id);
        if let Some(mut g) = self.guilds.get_mut(&guild_id) {
            g.whitelist.remove(&user_id);
        }
    }
}

impl Default for AntiNukeEngine {
    fn default() -> Self {
        Self::new()
    }
}
