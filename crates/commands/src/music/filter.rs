use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixContext;
use crate::core::traits::{AppCommand, PrefixCommand};
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_common::music_ipc::MusicCommand;
use redis::AsyncCommands;
use twilight_interactions::command::{CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::embed::EmbedBuilder;

#[derive(CommandOption, CreateOption)]
pub enum FilterType {
    #[option(name = "Bassboost", value = "bassboost")]
    Bassboost,
    #[option(name = "Nightcore", value = "nightcore")]
    Nightcore,
    #[option(name = "Vaporwave", value = "vaporwave")]
    Vaporwave,
    #[option(name = "8D", value = "8d")]
    EightD,
    #[option(name = "Studio (HQ)", value = "studio")]
    Studio,
    #[option(name = "Tremolo", value = "tremolo")]
    Tremolo,
    #[option(name = "Vibrato", value = "vibrato")]
    Vibrato,
    #[option(name = "Clear", value = "clear")]
    Clear,
}

impl FilterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bassboost => "bassboost",
            Self::Nightcore => "nightcore",
            Self::Vaporwave => "vaporwave",
            Self::EightD => "8d",
            Self::Studio => "studio",
            Self::Tremolo => "tremolo",
            Self::Vibrato => "vibrato",
            Self::Clear => "clear",
        }
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "filter", desc = "Apply an audio filter")]
pub struct FilterCommand {
    #[command(rename = "type", desc = "The filter to apply")]
    pub r#type: FilterType,
}

pub struct FilterAppCommand;
#[async_trait::async_trait]
impl AppCommand for FilterAppCommand {
    fn name(&self) -> &'static str {
        "filter"
    }
    fn register(&self) -> twilight_model::application::command::Command {
        FilterCommand::create_command().into()
    }
    async fn handle(
        &self,
        ctx: &InteractionContext,
        data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        let cmd = FilterCommand::from_interaction(data.clone().into())
            .map_err(|e| HarmonyError::Internal(e.to_string()))?;
        handle_filter(ctx, cmd, module_ctx).await
    }
}

pub struct FilterPrefixCommand;
#[async_trait::async_trait]
impl PrefixCommand for FilterPrefixCommand {
    fn aliases(&self) -> Vec<&'static str> {
        vec!["filter"]
    }
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_prefix_filter(ctx, module_ctx).await
    }
}

pub async fn handle_filter(
    interaction_ctx: &InteractionContext,
    cmd: FilterCommand,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let filter_type = cmd.r#type.as_str().to_string();

    let cmd =
        MusicCommand::Filter { guild_id: guild_id.to_string(), filter_type: filter_type.clone() };

    let payload = serde_json::to_string(&cmd).unwrap_or_default();
    let mut redis_conn = module_ctx.cache.clone();
    let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;

    let emoji = if filter_type == "clear" { "🧹" } else { "🎛️" };
    let msg = if filter_type == "clear" {
        "Cleared all audio filters.".to_string()
    } else {
        format!("Applied **{}** filter.", filter_type)
    };

    let embed = EmbedBuilder::new()
        .description(format!("{} {}", emoji, msg))
        .color(module_ctx.embed_color)
        .build();

    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_prefix_filter(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let filter = ctx.args.join(" ");

    let filter_type = filter.to_lowercase();
    let valid_filters = [
        "nightcore",
        "bassboost",
        "vaporwave",
        "8d",
        "karaoke",
        "tremolo",
        "vibrato",
        "pop",
        "soft",
        "treblebass",
        "echo",
        "chorus",
        "flanger",
        "gate",
        "haas",
        "phaser",
        "compressor",
        "expander",
        "lowpass",
        "highpass",
        "none",
        "clear",
        "reset",
        "",
    ];

    if !valid_filters.contains(&filter_type.as_str()) {
        let embed = EmbedBuilder::new().description("❌ Unknown filter!").color(0xFF0000).build();
        let _ = ctx.reply_embed(embed, module_ctx).await;
        return Ok(());
    }

    let payload = serde_json::to_string(&MusicCommand::Filter {
        guild_id: ctx.guild_id.to_string(),
        filter_type,
    })
    .unwrap_or_default();

    {
        use redis::AsyncCommands;
        let mut redis_conn = module_ctx.cache.clone();
        let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;
    }

    let msg = if filter.is_empty() || filter == "none" {
        "🎛️ Cleared all audio filters".to_string()
    } else {
        format!("🎛️ Applied filter: **{}**", filter)
    };

    let embed = EmbedBuilder::new().description(msg).color(module_ctx.embed_color).build();
    let _ = ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}
