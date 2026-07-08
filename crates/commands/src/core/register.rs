use harmony_common::error::HarmonyError;
use std::sync::Arc;
use tokio::sync::OnceCell;
use twilight_http::Client as HttpClient;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::{CommandBuilder, IntegerBuilder, StringBuilder};

static APP_ID: OnceCell<Id<ApplicationMarker>> = OnceCell::const_new();

pub async fn register_global_commands(http: Arc<HttpClient>) -> Result<(), HarmonyError> {
    let app_info = http.current_user_application().await?.model().await?;
    let app_id = app_info.id;
    APP_ID.set(app_id).ok();

    let interaction_client = http.interaction(app_id);

    let commands = vec![
        CommandBuilder::new(
            "play",
            "Play a song from YouTube/Spotify",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .option(StringBuilder::new("query", "The song name or URL to play").required(true))
        .build(),
        CommandBuilder::new(
            "stop",
            "Stop playback and clear the queue",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .build(),
        CommandBuilder::new(
            "skip",
            "Skip the current track",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .build(),
        CommandBuilder::new(
            "pause",
            "Pause the current track",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .build(),
        CommandBuilder::new(
            "resume",
            "Resume the paused track",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .build(),
        CommandBuilder::new(
            "queue",
            "View the current queue",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .build(),
        CommandBuilder::new(
            "volume",
            "Set the player volume (0-200)",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .option(IntegerBuilder::new("level", "Volume level percentage").required(true))
        .build(),
        CommandBuilder::new(
            "filter",
            "Apply an audio filter",
            twilight_model::application::command::CommandType::ChatInput,
        )
        .option(StringBuilder::new("type", "The filter to apply").required(true).choices(vec![
            ("Bassboost", "bassboost"),
            ("Nightcore", "nightcore"),
            ("Vaporwave", "vaporwave"),
            ("8D", "8d"),
            ("Studio (HQ)", "studio"),
            ("Tremolo", "tremolo"),
            ("Vibrato", "vibrato"),
            ("Clear", "clear"),
        ]))
        .build(),
    ];

    interaction_client.set_global_commands(&commands).await?;

    tracing::info!("Registered global application commands successfully");
    Ok(())
}
