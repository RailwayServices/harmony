use harmony_common::config::AppConfig;
use harmony_common::error::HarmonyError;
use harmony_gateway::dispatcher::EventDispatcher;
use harmony_gateway::event_loop::EventLoop;
use harmony_gateway::shard_manager::ShardManager;
use harmony_messaging::subscriber::Subscriber;
use harmony_messaging::transport::redis_transport::RedisTransport;
use lavende::LavendeManager;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use twilight_http::Client as DiscordClient;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), HarmonyError> {
    rustls::crypto::ring::default_provider().install_default().ok();
    dotenvy::dotenv().ok();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let config = AppConfig::from_env()?;

    info!("[SYSTEM] Starting Harmony Gateway Service...");

    let discord = Arc::new(DiscordClient::new(config.discord_token.clone()));

    info!("[TRANSPORT] Initializing Redis Transport for Gateway...");
    let redis_transport = RedisTransport::new(
        &config.redis_url,
        "harmony_events_discord",
        "harmony_events_worker",
        config.event_bus_capacity,
    )?;

    let (event_tx, event_rx) = tokio::sync::mpsc::channel(config.event_bus_capacity);
    let mut redis_rx = redis_transport.subscribe().await?;
    let event_tx_loop = event_tx.clone();
    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match redis_rx.recv().await {
                Ok(event) => {
                    if let Err(e) = event_tx_loop.send((*event).clone()).await {
                        tracing::error!("[GATEWAY] Failed to send event to EventLoop: {}", e);
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!("[GATEWAY] Redis subscriber lagged by {} messages", n);
                }
                Err(RecvError::Closed) => {
                    tracing::error!("[GATEWAY] Redis subscriber closed unexpectedly.");
                    break;
                }
            }
        }
    });

    info!("[GATEWAY] Initializing Lavende Audio Engine...");
    let current_user = discord
        .current_user()
        .await
        .map_err(|e| HarmonyError::Internal(e.to_string()))?
        .model()
        .await
        .map_err(|e| HarmonyError::Internal(e.to_string()))?;
    let client_id = current_user.id.to_string();

    let event_tx_clone = event_tx.clone();
    let send_to_shard_fn = move |guild_id: String, payload: serde_json::Value| {
        if let Ok(id) = guild_id.parse::<u64>() {
            let guild_id_marker = twilight_model::id::Id::new(id);
            let event = harmony_common::event::HarmonyEvent::SendToShard {
                guild_id: guild_id_marker,
                payload: serde_json::to_string(&payload).unwrap_or_default(),
            };
            let _ = event_tx_clone.try_send(event);
        }
    };
    let lavende_manager = Arc::new(LavendeManager::new(client_id, send_to_shard_fn));

    let audio_listener = harmony_gateway::audio_listener::AudioListener::new(
        lavende_manager.clone(),
        config.redis_url.clone(),
    );
    tokio::spawn(async move {
        audio_listener.run().await;
    });

    info!("[GATEWAY] Initializing Shard Manager...");
    let shard_manager = ShardManager::new(config.discord_token.clone(), &discord).await?;
    let dispatcher = EventDispatcher::new(Arc::new(redis_transport));
    let event_loop = EventLoop::new(
        shard_manager,
        dispatcher,
        event_rx,
        config.max_event_tasks,
        lavende_manager.clone(),
    );

    tokio::spawn(async move {
        if let Err(e) = event_loop.run().await {
            error!("[GATEWAY] Gateway event loop crashed: {}", e);
        }
    });

    info!("[SYSTEM] Harmony Gateway Service running. Waiting for shutdown signal...");

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("[SYSTEM] Shutdown signal received. Exiting gracefully.");
        }
        Err(err) => error!("[SYSTEM] Unable to listen for shutdown signal: {}", err),
    }

    Ok(())
}
