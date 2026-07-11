use dashmap::DashMap;
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

enum SpeakerState {
    Quiet(std::time::Instant),
    Speaking {
        speech_start: std::time::Instant,
        last_packet: std::time::Instant,
        auto_paused: bool,
    },
}

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
    let lavende_manager = Arc::new(LavendeManager::new(client_id.clone(), send_to_shard_fn));

    let audio_listener = harmony_gateway::audio_listener::AudioListener::new(
        lavende_manager.clone(),
        config.redis_url.clone(),
    );
    tokio::spawn(async move {
        audio_listener.run().await;
    });

    let mut event_rx_lavende = lavende_manager.subscribe_events();
    let redis_url_clone = config.redis_url.clone();
    let discord_clone = discord.clone();
    let lavende_manager_event_loop = lavende_manager.clone();
    let client_id_loop = client_id.clone();

    let active_speakers: Arc<DashMap<String, DashMap<String, SpeakerState>>> =
        Arc::new(DashMap::new());

    let lav_mgr_clone = lavende_manager.clone();
    let speakers_clone = active_speakers.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            interval.tick().await;

            let mut to_restore_volume = Vec::new();
            let mut to_remove = Vec::new();

            for guild_entry in speakers_clone.iter() {
                let guild_id = guild_entry.key().clone();
                let users = guild_entry.value();
                let mut is_anyone_speaking = false;
                let mut most_recent_quiet = None;

                for mut user_entry in users.iter_mut() {
                    let mut stopped = false;
                    match *user_entry.value() {
                        SpeakerState::Speaking { last_packet, .. } => {
                            if last_packet.elapsed().as_millis() > 400 {
                                stopped = true;
                            } else {
                                is_anyone_speaking = true;
                            }
                        }
                        SpeakerState::Quiet(time) => {
                            if most_recent_quiet.is_none() || Some(time) > most_recent_quiet {
                                most_recent_quiet = Some(time);
                            }
                        }
                    }
                    if stopped {
                        *user_entry.value_mut() = SpeakerState::Quiet(std::time::Instant::now());
                        to_restore_volume.push((guild_id.clone(), user_entry.key().clone()));
                        most_recent_quiet = Some(std::time::Instant::now());
                    }
                }

                if !is_anyone_speaking {
                    if let Some(time) = most_recent_quiet {
                        if time.elapsed().as_secs_f32() >= 5.0 {
                            to_remove.push(guild_id.clone());
                        }
                    } else {
                        to_remove.push(guild_id.clone());
                    }
                }
            }

            for (g_id, u_id) in to_restore_volume {
                tracing::info!("[AutoDuck] User {} stopped speaking in guild {}", u_id, g_id);

                let lav_mgr = lav_mgr_clone.clone();
                let speakers = speakers_clone.clone();

                tokio::spawn(async move {
                    let mut currently_speaking = false;
                    if let Some(guild_users) = speakers.get(&g_id) {
                        for user_state in guild_users.iter() {
                            if let SpeakerState::Speaking { .. } = *user_state.value() {
                                currently_speaking = true;
                                break;
                            }
                        }
                    }

                    if !currently_speaking {
                        tracing::info!(
                            "[AutoDuck] Silence confirmed in guild {}. Restoring volume to 100%.",
                            g_id
                        );
                        if let Some(player) = lav_mgr.get_player(&g_id) {
                            let _ = player.set_volume(100).await;
                        }
                    }
                });
            }

            for guild_id in to_remove {
                speakers_clone.remove(&guild_id);
            }
        }
    });

    tokio::spawn(async move {
        if let Ok(client) = redis::Client::open(redis_url_clone)
            && let Ok(mut con) = client.get_multiplexed_async_connection().await
                as Result<redis::aio::MultiplexedConnection, _>
        {
            use harmony_common::music_ipc::NowPlayingUiTemplate;
            use redis::AsyncCommands;
            use twilight_model::channel::message::EmojiReactionType;
            use twilight_model::channel::message::component::{
                ActionRow, Button, ButtonStyle, Component,
            };
            use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

            while let Ok(event) = event_rx_lavende.recv().await {
                match event {
                    lavende::LavendeEvent::TrackStart { guild_id, track } => {
                        let channel_id_str: Option<String> =
                            con.hget("harmony:player:channel", &guild_id).await.unwrap_or(None);
                        if let Some(c_id_str) = channel_id_str
                            && let Ok(channel_id) = c_id_str.parse::<u64>()
                        {
                            let channel_marker = twilight_model::id::Id::new(channel_id);

                            let old_msg_id_str: Option<String> =
                                con.hget("harmony:player:np_msg", &guild_id).await.unwrap_or(None);
                            if let Some(msg_id_str) = old_msg_id_str
                                && let Ok(msg_id) = msg_id_str.parse::<u64>()
                            {
                                let _ = discord_clone
                                    .delete_message(
                                        channel_marker,
                                        twilight_model::id::Id::new(msg_id),
                                    )
                                    .await;
                            }

                            let template_str: Option<String> =
                                con.get("harmony:ui:nowplaying").await.unwrap_or(None);
                            if let Some(t_str) = template_str
                                && let Ok(template) =
                                    serde_json::from_str::<NowPlayingUiTemplate>(&t_str)
                            {
                                let desc = template
                                    .description
                                    .replace("{title}", &track.info.title)
                                    .replace("{url}", &track.info.uri.unwrap_or_default())
                                    .replace("{author}", &track.info.author);

                                let mut builder =
                                    EmbedBuilder::new().description(desc).color(template.color);

                                if let Some(thumb) = track.info.artwork_url
                                    && let Ok(src) = ImageSource::url(thumb)
                                {
                                    builder = builder.thumbnail(src);
                                }
                                let embed = builder.build();

                                let mut components = Vec::new();
                                for btn in template.buttons {
                                    let style = match btn.style {
                                        1 => ButtonStyle::Primary,
                                        2 => ButtonStyle::Secondary,
                                        3 => ButtonStyle::Success,
                                        4 => ButtonStyle::Danger,
                                        _ => ButtonStyle::Secondary,
                                    };
                                    components.push(Component::Button(Button {
                                        id: None,
                                        custom_id: Some(btn.custom_id),
                                        disabled: false,
                                        emoji: Some(EmojiReactionType::Unicode { name: btn.emoji }),
                                        label: Some(btn.label),
                                        style,
                                        url: None,
                                        sku_id: None,
                                    }));
                                }

                                let action_row =
                                    vec![Component::ActionRow(ActionRow { id: None, components })];

                                if let Ok(msg) = discord_clone
                                    .create_message(channel_marker)
                                    .embeds(&[embed])
                                    .components(&action_row)
                                    .await
                                    && let Ok(msg_model) = msg.model().await
                                {
                                    let _: Result<(), _> = con
                                        .hset(
                                            "harmony:player:np_msg",
                                            &guild_id,
                                            msg_model.id.get().to_string(),
                                        )
                                        .await;
                                }
                            }
                        }
                    }
                    lavende::LavendeEvent::VoiceData { guild_id, user_id, pcm_data } => {
                        // Do not process bot's own voice
                        if user_id == client_id_loop {
                            continue;
                        }

                        let mut sum_sq = 0.0;
                        for &sample in &pcm_data {
                            let s = sample as f32 / 32768.0;
                            sum_sq += s * s;
                        }
                        let rms = (sum_sq / pcm_data.len() as f32).sqrt();

                        // Strict threshold to ignore background noise
                        let is_loud = rms > 0.035;

                        let is_new = {
                            let guild_map = active_speakers.entry(guild_id.clone()).or_default();
                            let mut user_state = guild_map
                                .entry(user_id.clone())
                                .or_insert(SpeakerState::Quiet(std::time::Instant::now()));

                            let mut new = false;
                            match *user_state {
                                SpeakerState::Quiet(_) => {
                                    if is_loud {
                                        let now = std::time::Instant::now();
                                        *user_state = SpeakerState::Speaking {
                                            speech_start: now,
                                            last_packet: now,
                                            auto_paused: false,
                                        };
                                        new = true;
                                    }
                                }
                                SpeakerState::Speaking {
                                    speech_start,
                                    ref mut last_packet,
                                    ref mut auto_paused,
                                } => {
                                    if is_loud {
                                        *last_packet = std::time::Instant::now();

                                        if !*auto_paused && speech_start.elapsed().as_secs() >= 10 {
                                            tracing::info!(
                                                "[AutoPause] User {} spoke for >10s in guild {}. Pausing music.",
                                                user_id,
                                                guild_id
                                            );
                                            *auto_paused = true;
                                            if let Some(player) = lavende_manager_event_loop.get_player(&guild_id) {
                                                tokio::spawn(async move {
                                                    let _ = player.pause(true).await;
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            new
                        };

                        if is_new {
                            tracing::info!(
                                "[AutoDuck] User {} started speaking in guild {} (RMS: {:.4})",
                                user_id,
                                guild_id,
                                rms
                            );
                            if let Some(player) = lavende_manager_event_loop.get_player(&guild_id) {
                                tracing::info!(
                                    "[AutoDuck] Ducking volume to 15% for guild {}",
                                    guild_id
                                );
                                let _ = player.set_volume(15).await;
                            }
                        }
                    }
                    lavende::LavendeEvent::TrackEnd { guild_id, .. }
                    | lavende::LavendeEvent::PlayerDestroy { guild_id, .. } => {
                        let channel_id_str: Option<String> =
                            con.hget("harmony:player:channel", &guild_id).await.unwrap_or(None);
                        if let Some(c_id_str) = channel_id_str
                            && let Ok(channel_id) = c_id_str.parse::<u64>()
                        {
                            let channel_marker = twilight_model::id::Id::new(channel_id);
                            let old_msg_id_str: Option<String> =
                                con.hget("harmony:player:np_msg", &guild_id).await.unwrap_or(None);
                            if let Some(msg_id_str) = old_msg_id_str {
                                if let Ok(msg_id) = msg_id_str.parse::<u64>() {
                                    let _ = discord_clone
                                        .delete_message(
                                            channel_marker,
                                            twilight_model::id::Id::new(msg_id),
                                        )
                                        .await;
                                }
                                let _: Result<(), _> =
                                    con.hdel("harmony:player:np_msg", &guild_id).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    info!("[GATEWAY] Initializing Shard Manager...");
    let shard_manager = ShardManager::new(config.discord_token.clone(), &discord).await?;
    let dispatcher = EventDispatcher::new(Arc::new(redis_transport));

    let discord_interactions = discord.clone();
    let redis_url_interactions = config.redis_url.clone();
    let lavende_interactions = lavende_manager.clone();
    tokio::spawn(async move {
        if let Ok(client) = redis::Client::open(redis_url_interactions)
            && let Ok(mut pubsub) = client.get_async_pubsub().await
        {
            let _ = pubsub.subscribe("harmony_events_discord").await;
            use futures::StreamExt;
            let mut stream = pubsub.into_on_message();
            while let Some(msg) = stream.next().await {
                let payload: String = match msg.get_payload() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                if let Ok(harmony_common::event::HarmonyEvent::Discord(arc_event)) =
                    serde_json::from_str(&payload)
                    && let harmony_common::event::SerializableEvent::InteractionCreate(interaction) =
                        arc_event.as_ref()
                    && let Some(
                        twilight_model::application::interaction::InteractionData::MessageComponent(
                            comp,
                        ),
                    ) = &interaction.0.data
                    && (comp.custom_id == "music_stop" || comp.custom_id == "music_skip")
                    && let Some(guild_id) = interaction.0.guild_id
                {
                    if comp.custom_id == "music_stop" {
                        if let Some(player) = lavende_interactions.get_player(&guild_id.to_string())
                        {
                            let _ = player.stop().await;
                        }
                    } else if comp.custom_id == "music_skip"
                        && let Some(player) = lavende_interactions.get_player(&guild_id.to_string())
                    {
                        let _ = player.skip().await;
                    }

                    let interaction_client =
                        discord_interactions.interaction(interaction.0.application_id);
                    let response = twilight_model::http::interaction::InteractionResponse {
                                                kind: twilight_model::http::interaction::InteractionResponseType::DeferredUpdateMessage,
                                                data: None,
                                            };
                    let _ = interaction_client
                        .create_response(interaction.0.id, &interaction.0.token, &response)
                        .await;
                }
            }
        }
    });

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
