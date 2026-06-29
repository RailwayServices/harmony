#![allow(deprecated)]
pub mod content;
pub mod engine;
pub mod punishment;
pub mod scorer;
pub mod snapshot;
pub mod types;
pub mod whitelist;

use once_cell::sync::Lazy;
use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_common::module::Module;
use railway_database::pool::Database;

use std::sync::Arc;
use tracing::{debug, warn};
use twilight_http::request::AuditLogReason;
use twilight_http::Client as HttpClient;
use twilight_model::id::Id;

static ENGINE: Lazy<engine::AntiNukeEngine> = Lazy::new(engine::AntiNukeEngine::new);

static OWNER_CACHE: Lazy<dashmap::DashMap<u64, u64>> = Lazy::new(dashmap::DashMap::new);

pub async fn reload_guild_config(pool: &sqlx::PgPool, guild_id: u64) {
    let repo =
        railway_database::repository::antinuke_repository::AntinukeRepository::new(pool.clone());
    let guild_id_i64 = guild_id as i64;

    let db_config = match repo.get_config(guild_id_i64).await {
        Ok(Some(c)) => c,
        _ => return,
    };
    let db_modules = match repo.get_module_configs(guild_id_i64).await {
        Ok(m) => m,
        _ => return,
    };

    let mut modules = std::collections::HashMap::new();
    for m in db_modules {
        if let Some(action) = types::ActionType::parse(&m.action_type) {
            let punishment =
                types::Punishment::parse(&m.punishment).unwrap_or(types::Punishment::None);
            modules.insert(
                action as u8,
                types::InternalModuleConfig {
                    enabled: m.enabled,
                    threshold: m.threshold as u32,
                    window_secs: m.window_secs as u32,
                    punishment,
                    log_only: m.log_only,
                },
            );
        }
    }

    let config = types::GuildConfig {
        enabled: db_config.enabled,
        modules,
        log_channel_id: db_config.log_channel_id.map(|id| id as u64),
    };
    let whitelists = repo.get_whitelist(guild_id_i64).await.unwrap_or_default();
    ENGINE.configure(guild_id, config, whitelists.into_iter().map(|id| id as u64).collect());
    debug!("[ANTINUKE] Reloaded config for guild {} into engine", guild_id);
}

pub fn whitelist_add(guild_id: u64, user_id: u64) {
    ENGINE.whitelist_add(guild_id, user_id);
}

pub fn whitelist_remove(guild_id: u64, user_id: u64) {
    ENGINE.whitelist_remove(guild_id, user_id);
}

#[inline]
fn is_bot(user_id: u64) -> bool {
    let cached = railway_common::ids::get_bot_id();
    cached != 0 && cached == user_id
}

#[inline]
fn is_owner(guild_id: u64, user_id: u64) -> bool {
    OWNER_CACHE.get(&guild_id).map(|v| *v == user_id).unwrap_or(false)
}

#[inline]
fn should_skip(guild_id: u64, user_id: u64) -> bool {
    is_bot(user_id) || is_owner(guild_id, user_id)
}

fn restore_resource(http: &Arc<HttpClient>, guild_id: u64, resource_type: &str, resource_id: u64) {
    let http = Arc::clone(http);
    let resource_type = resource_type.to_string();
    let t_guild_id = Id::new(guild_id);

    tokio::spawn(async move {
        match resource_type.as_str() {
            "channel" => {
                if let Some(ch) = ENGINE.get_channel_snap(guild_id, resource_id) {
                    let mut req = http
                        .create_guild_channel(t_guild_id, &ch.name)
                        .kind(twilight_model::channel::ChannelType::from(ch.kind))
                        .position(ch.position as u64)
                        .nsfw(ch.nsfw)
                        .rate_limit_per_user(ch.rate_limit_per_user);
                    if let Some(t) = &ch.topic {
                        req = req.topic(t);
                    }
                    if let Some(pid) = ch.parent_id {
                        req = req.parent_id(Id::new(pid));
                    }
                    match req.reason("[Railway AntiNuke] Auto-restore").await {
                        Ok(_) => debug!(
                            "[ANTINUKE] Restored channel '{}' in guild {}",
                            ch.name, guild_id
                        ),
                        Err(e) => {
                            warn!("[ANTINUKE] Failed to restore channel '{}': {}", ch.name, e)
                        }
                    }
                }
            }
            "role" => {
                if let Some(r) = ENGINE.get_role_snap(guild_id, resource_id) {
                    if r.name == "@everyone" {
                        return;
                    }
                    match http
                        .create_role(t_guild_id)
                        .name(&r.name)
                        .color(r.color)
                        .permissions(twilight_model::guild::Permissions::from_bits_truncate(
                            r.permissions,
                        ))
                        .hoist(r.hoist)
                        .mentionable(r.mentionable)
                        .reason("[Railway AntiNuke] Auto-restore")
                        .await
                    {
                        Ok(_) => {
                            debug!("[ANTINUKE] Restored role '{}' in guild {}", r.name, guild_id)
                        }
                        Err(e) => {
                            warn!("[ANTINUKE] Failed to restore role '{}': {}", r.name, e)
                        }
                    }
                }
            }
            _ => {}
        }
    });
}

pub struct AntinukeModule {
    http: Arc<HttpClient>,
    db: Database,
}

impl AntinukeModule {
    pub fn new(http: Arc<HttpClient>, db: Database) -> Self {
        Self { http, db }
    }
}

impl Module for AntinukeModule {
    fn name(&self) -> &'static str {
        "AntinukeModule"
    }

    async fn handle_event(
        &self,
        event: &RailwayEvent,
        _ctx: &railway_common::module::ModuleContext,
    ) -> Result<(), RailwayError> {
        if let RailwayEvent::Discord(box_event) = event {
            match &**box_event {
                twilight_model::gateway::event::Event::GuildCreate(ev) => {
                    if let twilight_model::gateway::payload::incoming::GuildCreate::Available(
                        guild,
                    ) = ev.as_ref()
                    {
                        let guild_id = guild.id.get();

                        OWNER_CACHE.insert(guild_id, guild.owner_id.get());

                        if railway_common::ids::get_bot_id() == 0 {
                            if let Ok(resp) = self.http.current_user().await {
                                if let Ok(me) = resp.model().await {
                                    railway_common::ids::set_bot_id(me.id.get());
                                    ENGINE.whitelist_add(guild_id, me.id.get());
                                }
                            }
                        }

                        reload_guild_config(&self.db.pool, guild_id).await;

                        let channels: Vec<snapshot::ChannelSnap> = guild
                            .channels
                            .iter()
                            .map(|c| snapshot::ChannelSnap {
                                id: c.id.get(),
                                name: c.name.clone().unwrap_or_default(),
                                kind: c.kind.into(),
                                position: c.position.unwrap_or(0),
                                topic: c.topic.clone(),
                                nsfw: c.nsfw.unwrap_or(false),
                                rate_limit_per_user: c.rate_limit_per_user.unwrap_or(0),
                                parent_id: c.parent_id.map(|pid| pid.get()),
                                overwrites: vec![],
                            })
                            .collect();
                        let roles: Vec<snapshot::RoleSnap> = guild
                            .roles
                            .iter()
                            .map(|r| snapshot::RoleSnap {
                                id: r.id.get(),
                                name: r.name.clone(),
                                color: r.color,
                                permissions: r.permissions.bits(),
                                position: r.position,
                                hoist: r.hoist,
                                mentionable: r.mentionable,
                            })
                            .collect();
                        ENGINE.set_snapshot(snapshot::GuildSnapshot {
                            guild_id,
                            name: guild.name.clone(),
                            channels,
                            roles,
                        });

                        for member in &guild.members {
                            ENGINE.set_member_roles(
                                guild_id,
                                member.user.id.get(),
                                member.roles.iter().map(|r| r.get()).collect(),
                            );
                        }
                    }
                }

                twilight_model::gateway::event::Event::ChannelCreate(ev) => {
                    if let Some(gid) = ev.guild_id {
                        let snap = snapshot::ChannelSnap {
                            id: ev.id.get(),
                            name: ev.name.clone().unwrap_or_default(),
                            kind: ev.kind.into(),
                            position: ev.position.unwrap_or(0),
                            topic: ev.topic.clone(),
                            nsfw: ev.nsfw.unwrap_or(false),
                            rate_limit_per_user: ev.rate_limit_per_user.unwrap_or(0),
                            parent_id: ev.parent_id.map(|id| id.get()),
                            overwrites: vec![],
                        };
                        ENGINE.upsert_channel_snap(gid.get(), snap);
                    }
                }

                twilight_model::gateway::event::Event::ChannelDelete(_ev) => {}
                twilight_model::gateway::event::Event::RoleDelete(_ev) => {}

                twilight_model::gateway::event::Event::RoleCreate(ev) => {
                    let snap = snapshot::RoleSnap {
                        id: ev.role.id.get(),
                        name: ev.role.name.clone(),
                        color: ev.role.color,
                        permissions: ev.role.permissions.bits(),
                        position: ev.role.position,
                        hoist: ev.role.hoist,
                        mentionable: ev.role.mentionable,
                    };
                    ENGINE.upsert_role_snap(ev.guild_id.get(), snap);
                }

                twilight_model::gateway::event::Event::RoleUpdate(ev) => {
                    let snap = snapshot::RoleSnap {
                        id: ev.role.id.get(),
                        name: ev.role.name.clone(),
                        color: ev.role.color,
                        permissions: ev.role.permissions.bits(),
                        position: ev.role.position,
                        hoist: ev.role.hoist,
                        mentionable: ev.role.mentionable,
                    };
                    ENGINE.upsert_role_snap(ev.guild_id.get(), snap);
                }

                twilight_model::gateway::event::Event::MemberAdd(ev) => {
                    ENGINE.set_member_roles(
                        ev.guild_id.get(),
                        ev.user.id.get(),
                        ev.roles.iter().map(|r| r.get()).collect(),
                    );
                }

                twilight_model::gateway::event::Event::MemberUpdate(ev) => {
                    ENGINE.set_member_roles(
                        ev.guild_id.get(),
                        ev.user.id.get(),
                        ev.roles.iter().map(|r| r.get()).collect(),
                    );
                }

                twilight_model::gateway::event::Event::MessageCreate(ev) => {
                    if let Some(gid) = ev.guild_id {
                        if ev.author.bot {
                            return Ok(());
                        }
                        let matches =
                            ENGINE.scan_content(gid.get(), ev.author.id.get(), &ev.content, &[]);
                        let mut cache = _ctx.cache.clone();
                        for m in matches {
                            if let Some(result) = ENGINE
                                .process_event(
                                    gid.get(),
                                    ev.author.id.get(),
                                    m.module,
                                    None,
                                    &mut cache,
                                )
                                .await
                            {
                                if result.triggered {
                                    let _ = punishment::execute(
                                        &ENGINE,
                                        &self.http,
                                        &self.db,
                                        gid.get(),
                                        ev.author.id.get(),
                                        &result,
                                        &mut cache,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                }

                twilight_model::gateway::event::Event::GuildAuditLogEntryCreate(ev) => {
                    if let Some(gid) = ev.guild_id {
                        let action = match ev.action_type {
                            twilight_model::guild::audit_log::AuditLogEventType::ChannelDelete => {
                                Some(types::ActionType::ChannelDelete)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::ChannelCreate => {
                                Some(types::ActionType::ChannelCreate)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::RoleDelete => {
                                Some(types::ActionType::RoleDelete)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::RoleCreate => {
                                Some(types::ActionType::RoleCreate)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::MemberBanAdd => {
                                Some(types::ActionType::BanAdd)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::MemberKick => {
                                Some(types::ActionType::MemberKick)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::BotAdd => {
                                Some(types::ActionType::BotAdd)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::RoleUpdate => {
                                Some(types::ActionType::RoleUpdate)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::ChannelUpdate => {
                                Some(types::ActionType::ChannelUpdate)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::EmojiDelete => {
                                Some(types::ActionType::EmojiDelete)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::StickerDelete => {
                                Some(types::ActionType::StickerDelete)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::WebhookCreate => {
                                Some(types::ActionType::WebhookCreate)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::GuildUpdate => {
                                Some(types::ActionType::GuildUpdate)
                            }
                            twilight_model::guild::audit_log::AuditLogEventType::MemberPrune => {
                                Some(types::ActionType::MemberPrune)
                            }
                            _ => None,
                        };

                        if let Some(act) = action {
                            if let Some(user_id) = ev.user_id {
                                let uid = user_id.get();
                                let gid_val = gid.get();

                                if should_skip(gid_val, uid) {
                                    return Ok(());
                                }

                                let mut cache = _ctx.cache.clone();
                                if let Some(result) =
                                    ENGINE.process_event(gid_val, uid, act, None, &mut cache).await
                                {
                                    if result.triggered {
                                        if !ENGINE.try_claim_punishment(gid_val, uid) {
                                            return Ok(());
                                        }

                                        let needs_restore = result.should_restore;
                                        let target = ev.target_id.map(|t| t.get());

                                        let http = self.http.clone();
                                        let db = self.db.clone();
                                        let mut redis_clone = _ctx.cache.clone();

                                        tokio::spawn(async move {
                                            if needs_restore {
                                                if let Some(target_id) = target {
                                                    match act {
                                                        types::ActionType::ChannelDelete => {
                                                            restore_resource(
                                                                &http, gid_val, "channel",
                                                                target_id,
                                                            );
                                                        }
                                                        types::ActionType::RoleDelete => {
                                                            restore_resource(
                                                                &http, gid_val, "role", target_id,
                                                            );
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }

                                            let _ = punishment::execute(
                                                &ENGINE,
                                                &http,
                                                &db,
                                                gid_val,
                                                uid,
                                                &result,
                                                &mut redis_clone,
                                            )
                                            .await;
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
