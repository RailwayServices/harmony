use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{
    content::{scan, ContentScanInput},
    scorer::{compute_score, has_dangerous_perm_grant, resolve_punishment},
    snapshot::{ChannelSnap, GuildSnapshot, RoleSnap, SnapshotStore},
    types::{ActionType, ContentMatch, GuildConfig, Punishment, ThreatResult},
    whitelist::WhitelistStore,
    window::{now_millis_cached, start_clock_ticker, ActionWindow},
};

struct GuildState {
    config: ArcSwap<GuildConfig>,
    user_windows: DashMap<u64, HashMap<u8, ActionWindow>>,
    whitelist: HashSet<u64>,
}

pub struct AntiNukeEngine {
    guilds: DashMap<u64, GuildState>,
    snapshots: SnapshotStore,
    whitelist_store: WhitelistStore,
}

impl AntiNukeEngine {
    #[must_use]
    pub fn new() -> Self {
        start_clock_ticker();
        Self {
            guilds: DashMap::new(),
            snapshots: SnapshotStore::new(),
            whitelist_store: WhitelistStore::new(),
        }
    }

    pub fn configure(&self, guild_id: u64, config: GuildConfig, whitelist_ids: Vec<u64>) {
        let whitelist: HashSet<u64> = whitelist_ids.iter().copied().collect();
        self.whitelist_store.set(guild_id, whitelist_ids);

        if let Some(state) = self.guilds.get(&guild_id) {
            state.config.store(Arc::new(config));
        } else {
            let state = GuildState {
                config: ArcSwap::from_pointee(config),
                user_windows: DashMap::new(),
                whitelist,
            };
            self.guilds.insert(guild_id, state);
        }
    }

    pub fn remove_guild(&self, guild_id: u64) {
        self.guilds.remove(&guild_id);
        self.snapshots.remove_guild(guild_id);
        self.whitelist_store.remove_guild(guild_id);
    }

    pub fn clear_user(&self, guild_id: u64, user_id: u64) {
        if let Some(state) = self.guilds.get(&guild_id) {
            state.user_windows.remove(&user_id);
        }
    }

    pub fn process_event(
        &self,
        guild_id: u64,
        user_id: u64,
        action: ActionType,
        extra_data: Option<&(u64, u64)>,
    ) -> Option<ThreatResult> {
        let guild = self.guilds.get(&guild_id)?;
        let config = guild.config.load();

        if !config.enabled {
            return None;
        }

        if action.is_content() {
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

        let now_ms = now_millis_cached();

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

        let count = {
            let mut user_windows = guild.user_windows.entry(user_id).or_default();
            user_windows
                .entry(action as u8)
                .or_insert_with(|| ActionWindow::new(module_cfg.window_secs))
                .push_and_count(now_ms)
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

    pub fn process_content_match(
        &self,
        guild_id: u64,
        user_id: u64,
        module: ActionType,
    ) -> Option<ThreatResult> {
        self.process_event(guild_id, user_id, module, None)
    }

    pub fn set_snapshot(&self, snap: GuildSnapshot) {
        self.snapshots.set(snap);
    }

    #[must_use]
    pub fn get_snapshot(&self, guild_id: u64) -> Option<GuildSnapshot> {
        self.snapshots.get(guild_id)
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
