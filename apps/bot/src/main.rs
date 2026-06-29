use railway_commands::router::CommandRouter;
use railway_common::config::AppConfig;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::pool::Database;
use railway_gateway::dispatcher::EventDispatcher;
use railway_gateway::event_loop::EventLoop;
use railway_gateway::shard_manager::ShardManager;
use railway_messaging::subscriber::Subscriber;
use railway_messaging::transport::local_transport::LocalTransport;
use railway_modules_registry::ModuleRegistry;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};
use twilight_http::Client as DiscordClient;

fn sanitize_postgres_url(url: &str) -> String {
    if let Some(pos) = url.find('@') {
        let host_part = &url[pos + 1..];
        format!("postgresql://*****@{}", host_part)
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

#[tokio::main]
async fn main() -> Result<(), RailwayError> {
    rustls::crypto::ring::default_provider().install_default().ok();
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_env()?;

    info!("[POSTGRES] Connecting to database: {}", sanitize_postgres_url(&config.database_url));
    let db_wrapper = Database::connect(&config.database_url).await?;
    let db = db_wrapper.pool.clone();

    info!("[POSTGRES] Running database migrations...");
    sqlx::migrate!("../../migrations").run(&db).await?;
    info!("[POSTGRES] Migrations complete.");

    info!("[REDIS] Connecting to server: {}", sanitize_redis_url(&config.redis_url));
    let redis_client =
        redis::Client::open(config.redis_url.clone()).map_err(RailwayError::Cache)?;
    let cache =
        redis_client.get_multiplexed_async_connection().await.map_err(RailwayError::Cache)?;

    let discord = Arc::new(DiscordClient::new(config.discord_token.clone()));

    let bot_user = discord.current_user().await?.model().await?;
    let bot_name = bot_user.name.clone();
    railway_common::ids::set_bot_id(bot_user.id.get());

    info!("[DISCORD] Registering global slash commands...");
    railway_commands::register::register_global_commands(discord.clone()).await?;
    info!("[DISCORD] Global slash commands registered successfully");

    info!("[TRANSPORT] Initializing local messaging transport (Buffer: 1024)...");
    let local_transport = Arc::new(LocalTransport::new(1024));

    let module_ctx =
        Arc::new(ModuleContext { db, cache, discord, embed_color: config.embed_color });

    let registry = Arc::new(ModuleRegistry::new(module_ctx.discord.clone(), db_wrapper.clone()));
    let mut rx = local_transport.subscribe().await?;
    let registry_ctx = module_ctx.clone();

    tokio::spawn(async move {
        info!("[MODULES] Module registry listening for events...");
        while let Ok(event) = rx.recv().await {
            let registry = registry.clone();
            let registry_ctx = registry_ctx.clone();
            tokio::spawn(async move {
                if let Err(e) = registry.handle_event(&event, &registry_ctx).await {
                    error!("[MODULES] Failed to handle event: {}", e);
                }
            });
        }
    });

    let command_router = Arc::new(CommandRouter::new(config.prefix.clone()));
    let mut cmd_rx = local_transport.subscribe().await?;
    let cmd_ctx = module_ctx.clone();

    tokio::spawn(async move {
        info!("[COMMANDS] Router listening for events...");
        while let Ok(event) = cmd_rx.recv().await {
            let command_router = command_router.clone();
            let cmd_ctx = cmd_ctx.clone();
            tokio::spawn(async move {
                if let Err(e) = command_router.handle_event(&event, &cmd_ctx).await {
                    error!("[COMMANDS] Router failed to handle event: {}", e);
                }
            });
        }
    });

    info!("[GATEWAY] Initializing Shard Manager with recommended sharding...");
    let shard_manager =
        ShardManager::new(config.discord_token.clone(), &module_ctx.discord).await?;
    let dispatcher = EventDispatcher::new(local_transport.clone());
    let event_loop = EventLoop::new(shard_manager, dispatcher);

    tokio::spawn(async move {
        if let Err(e) = event_loop.run().await {
            error!("[GATEWAY] Gateway event loop crashed: {}", e);
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    info!("[SYSTEM] Connected to Discord as {}", bot_name);
    info!("[SYSTEM] Waiting for shutdown signal...");

    match signal::ctrl_c().await {
        Ok(()) => info!("[SYSTEM] Shutdown signal received. Exiting gracefully..."),
        Err(err) => error!("[SYSTEM] Unable to listen for shutdown signal: {}", err),
    }

    Ok(())
}
