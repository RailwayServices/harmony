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

#[tokio::main]
async fn main() -> Result<(), RailwayError> {
    tracing_subscriber::fmt::init();
    info!("Starting Railway bot process...");

    let config = AppConfig::from_env()?;

    info!("Connecting to PostgreSQL...");
    let db_wrapper = Database::connect(&config.database_url).await?;
    let db = db_wrapper.pool.clone();

    info!("Connecting to Redis...");
    let cache = redis::Client::open(config.redis_url.clone()).map_err(RailwayError::Cache)?;

    let discord = Arc::new(DiscordClient::new(config.discord_token.clone()));

    info!("Registering slash commands...");
    railway_commands::register::register_global_commands(discord.clone()).await?;

    info!("Initializing local messaging transport...");
    let local_transport = Arc::new(LocalTransport::new(1024));

    let module_ctx = Arc::new(ModuleContext { db, cache, discord });

    let registry = Arc::new(ModuleRegistry::new(module_ctx.discord.clone(), db_wrapper.clone()));
    let mut rx = local_transport.subscribe().await?;
    let registry_ctx = module_ctx.clone();

    tokio::spawn(async move {
        info!("Module registry listening for events...");
        while let Ok(event) = rx.recv().await {
            if let Err(e) = registry.handle_event(&event, &registry_ctx).await {
                error!("Module registry failed to handle event: {}", e);
            }
        }
    });

    let command_router = Arc::new(CommandRouter::new(config.prefix.clone()));
    let mut cmd_rx = local_transport.subscribe().await?;
    let cmd_ctx = module_ctx.clone();

    tokio::spawn(async move {
        info!("Command Router listening for events...");
        while let Ok(event) = cmd_rx.recv().await {
            if let Err(e) = command_router.handle_event(&event, &cmd_ctx).await {
                error!("Command Router failed to handle event: {}", e);
            }
        }
    });

    info!("Initializing Shard Manager...");
    let shard_manager = ShardManager::new(config.discord_token.clone())?;
    let dispatcher = EventDispatcher::new(local_transport.clone());
    let event_loop = EventLoop::new(shard_manager, dispatcher);

    tokio::spawn(async move {
        if let Err(e) = event_loop.run().await {
            error!("Gateway event loop crashed: {}", e);
        }
    });

    info!("Railway is fully operational. Waiting for SIGINT...");

    match signal::ctrl_c().await {
        Ok(()) => info!("Shutdown signal received. Exiting gracefully..."),
        Err(err) => error!("Unable to listen for shutdown signal: {}", err),
    }

    Ok(())
}
