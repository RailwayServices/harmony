use harmony_commands::core::router::CommandRouter;
use harmony_common::config::AppConfig;
use harmony_common::error::HarmonyError;
use harmony_common::module::{Module, ModuleContext};
use harmony_database::pool::Database;
use harmony_messaging::subscriber::Subscriber;
use harmony_messaging::transport::redis_transport::RedisTransport;
use harmony_modules::{MusicModule, LAVENDE_MANAGER};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use twilight_http::Client as DiscordClient;

const CONSUMER_CONCURRENCY: usize = 256;

fn sanitize_postgres_url(url: &str) -> String {
    if let Some(pos) = url.find('@') {
        format!("postgresql://*****@{}", &url[pos + 1..])
    } else {
        url.to_string()
    }
}

fn sanitize_redis_url(url: &str) -> String {
    if let Some(pos) = url.find("://") {
        let scheme = &url[..pos + 3];
        let rest = &url[pos + 3..];
        if let Some(at_pos) = rest.find('@') {
            format!("{}*****@{}", scheme, &rest[at_pos + 1..])
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), HarmonyError> {
    rustls::crypto::ring::default_provider().install_default().ok();
    dotenvy::dotenv().ok();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let config = AppConfig::from_env()?;

    info!("[POSTGRES] Connecting to database: {}", sanitize_postgres_url(&config.database_url));
    let db_wrapper = Database::connect(&config.database_url, config.db_pool_size).await?;
    let db = db_wrapper.pool.clone();

    info!("[POSTGRES] Running database migrations...");
    db_wrapper.run_migrations().await.map_err(|e| {
        error!("[POSTGRES] Failed to run migrations: {}", e);
        HarmonyError::Database(e)
    })?;
    info!("[POSTGRES] Migrations complete.");

    info!("[REDIS] Connecting to server: {}", sanitize_redis_url(&config.redis_url));
    let redis_client =
        redis::Client::open(config.redis_url.clone()).map_err(HarmonyError::Cache)?;
    let cache =
        redis_client.get_multiplexed_async_connection().await.map_err(HarmonyError::Cache)?;

    let discord = Arc::new(DiscordClient::new(config.discord_token.clone()));

    let bot_user = discord.current_user().await?.model().await?;
    let bot_name = bot_user.name.clone();
    harmony_common::ids::set_bot_id(bot_user.id.get());

    info!("[DISCORD] Registering global slash commands...");
    harmony_commands::core::register::register_global_commands(discord.clone()).await?;
    info!("[DISCORD] Global slash commands registered successfully");

    info!(
        "[TRANSPORT] Initializing Redis messaging transport (capacity: {})...",
        config.event_bus_capacity
    );
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(config.event_bus_capacity);
    let redis_transport = Arc::new(RedisTransport::new(
        &config.redis_url,
        "harmony_events_worker",
        "harmony_events_discord",
        config.event_bus_capacity,
    )?);

    let module_ctx = Arc::new(ModuleContext {
        db,
        cache,
        discord,
        embed_color: config.embed_color,
        event_tx: event_tx.clone(),
    });

    let registry = Arc::new(MusicModule::new(module_ctx.clone()));

    {
        use redis::AsyncCommands;
        let mut redis_conn = module_ctx.cache.clone();
        if let Ok(keys) = redis_conn.keys::<_, Vec<String>>("harmony:player_state:*").await {
            let manager_opt = LAVENDE_MANAGER.get();
            if let Some(manager) = manager_opt {
                let dummy = manager.get_or_create_player("1");
                tokio::spawn(async move {
                    let _ = dummy.search("warmup").await;
                    manager.destroy_player("1").await;
                });

                for key in keys {
                    if let Ok(json_str) = redis_conn.get::<_, String>(&key).await {
                        if let Ok(payload) = serde_json::from_str::<
                            harmony_modules::state_sync::PlayerStatePayload,
                        >(&json_str)
                        {
                            let guild_id = payload.guild_id.clone();
                            let player = manager.get_or_create_player(&guild_id);
                            let was_paused = payload.paused;
                            let voice_channel_id = payload.voice_channel_id.clone();
                            harmony_modules::state_sync::restore_player_state(
                                &guild_id, &player, payload,
                            )
                            .await;
                            tracing::info!(
                                "[STATE_SYNC] Restored player state for guild {}",
                                guild_id
                            );

                            if let Some(vc_id) = voice_channel_id {
                                let p = player.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                    let _ = p.connect(Some(vc_id), true, false).await;

                                    if !was_paused {
                                        tokio::time::sleep(std::time::Duration::from_millis(1000))
                                            .await;
                                        let _ = p.play().await;
                                    }
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let mut rx = redis_transport.subscribe().await?;
    let registry_ctx = module_ctx.clone();
    let module_sem = Arc::new(Semaphore::new(CONSUMER_CONCURRENCY));

    tokio::spawn(async move {
        info!(
            "[MODULES] Module registry listening for events (max concurrency: {})...",
            CONSUMER_CONCURRENCY
        );
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let Ok(permit) = module_sem.clone().try_acquire_owned() else {
                        warn!("[MODULES] Concurrency limit reached — dropping event.");
                        continue;
                    };
                    let registry = registry.clone();
                    let registry_ctx = registry_ctx.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(e) = registry.handle_event(&event, &registry_ctx).await {
                            error!("[MODULES] Failed to handle event: {}", e);
                        }
                    });
                }
                Err(RecvError::Lagged(n)) => {
                    warn!("[MODULES] Event bus lagged — {} events dropped. Increase EVENT_BUS_CAPACITY.", n);
                }
                Err(RecvError::Closed) => {
                    error!("[MODULES] Event bus closed unexpectedly.");
                    break;
                }
            }
        }
    });

    let command_router = Arc::new(CommandRouter::new(config.prefix.clone()));
    let mut cmd_rx = redis_transport.subscribe().await?;
    let cmd_ctx = module_ctx.clone();
    let cmd_sem = Arc::new(Semaphore::new(CONSUMER_CONCURRENCY));

    tokio::spawn(async move {
        info!(
            "[COMMANDS] Router listening for events (max concurrency: {})...",
            CONSUMER_CONCURRENCY
        );
        loop {
            match cmd_rx.recv().await {
                Ok(event) => {
                    let Ok(permit) = cmd_sem.clone().try_acquire_owned() else {
                        warn!("[COMMANDS] Concurrency limit reached — dropping event.");
                        continue;
                    };
                    let command_router = command_router.clone();
                    let cmd_ctx = cmd_ctx.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(e) = command_router.handle_event(&event, &cmd_ctx).await {
                            error!("[COMMANDS] Router failed to handle event: {}", e);
                        }
                    });
                }
                Err(RecvError::Lagged(n)) => {
                    warn!("[COMMANDS] Event bus lagged — {} events dropped. Increase EVENT_BUS_CAPACITY.", n);
                }
                Err(RecvError::Closed) => {
                    error!("[COMMANDS] Event bus closed unexpectedly.");
                    break;
                }
            }
        }
    });

    let redis_publisher = redis_transport.clone();
    tokio::spawn(async move {
        let mut forward_rx = event_rx;
        use harmony_messaging::publisher::Publisher;
        while let Some(event) = forward_rx.recv().await {
            if let Err(e) = redis_publisher.publish(Arc::new(event)).await {
                error!("[WORKER] Failed to publish event to Redis: {}", e);
            }
        }
    });

    info!("[SYSTEM] Connected to Discord as {} (Worker Node)", bot_name);
    info!("[SYSTEM] Waiting for shutdown signal...");

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("[SYSTEM] Shutdown signal received. Destroying all active players...");
            if let Some(manager) = LAVENDE_MANAGER.get() {
                let guild_ids: Vec<String> =
                    manager.players.iter().map(|e| e.key().clone()).collect();
                for guild_id in guild_ids {
                    manager.destroy_player(&guild_id).await;
                }
                info!("[SYSTEM] All players destroyed.");
            }
            info!("[SYSTEM] Exiting gracefully.");
        }
        Err(err) => error!("[SYSTEM] Unable to listen for shutdown signal: {}", err),
    }

    Ok(())
}
