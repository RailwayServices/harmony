use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixContext;
use crate::core::traits::{AppCommand, PrefixCommand};
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_common::music_ipc::MusicCommand;
use redis::AsyncCommands;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::embed::EmbedBuilder;

#[derive(CommandModel, CreateCommand)]
#[command(name = "stop", desc = "Stop playback and clear the queue")]
pub struct StopCommand {}

pub struct StopAppCommand;
#[async_trait::async_trait]
impl AppCommand for StopAppCommand {
    fn name(&self) -> &'static str {
        "stop"
    }
    fn register(&self) -> twilight_model::application::command::Command {
        StopCommand::create_command().into()
    }
    async fn handle(
        &self,
        ctx: &InteractionContext,
        _data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_stop(ctx, module_ctx).await
    }
}
pub struct StopPrefixCommand;
#[async_trait::async_trait]
impl PrefixCommand for StopPrefixCommand {
    fn aliases(&self) -> Vec<&'static str> {
        vec!["stop"]
    }
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_prefix_stop(ctx, module_ctx).await
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "skip", desc = "Skip the current track")]
pub struct SkipCommand {}

pub struct SkipAppCommand;
#[async_trait::async_trait]
impl AppCommand for SkipAppCommand {
    fn name(&self) -> &'static str {
        "skip"
    }
    fn register(&self) -> twilight_model::application::command::Command {
        SkipCommand::create_command().into()
    }
    async fn handle(
        &self,
        ctx: &InteractionContext,
        _data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_skip(ctx, module_ctx).await
    }
}
pub struct SkipPrefixCommand;
#[async_trait::async_trait]
impl PrefixCommand for SkipPrefixCommand {
    fn aliases(&self) -> Vec<&'static str> {
        vec!["skip", "s", "next"]
    }
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_prefix_skip(ctx, module_ctx).await
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "pause", desc = "Pause the current track")]
pub struct PauseCommand {}

pub struct PauseAppCommand;
#[async_trait::async_trait]
impl AppCommand for PauseAppCommand {
    fn name(&self) -> &'static str {
        "pause"
    }
    fn register(&self) -> twilight_model::application::command::Command {
        PauseCommand::create_command().into()
    }
    async fn handle(
        &self,
        ctx: &InteractionContext,
        _data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_pause(ctx, module_ctx).await
    }
}
pub struct PausePrefixCommand;
#[async_trait::async_trait]
impl PrefixCommand for PausePrefixCommand {
    fn aliases(&self) -> Vec<&'static str> {
        vec!["pause"]
    }
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_prefix_pause(ctx, module_ctx).await
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "resume", desc = "Resume the paused track")]
pub struct ResumeCommand {}

pub struct ResumeAppCommand;
#[async_trait::async_trait]
impl AppCommand for ResumeAppCommand {
    fn name(&self) -> &'static str {
        "resume"
    }
    fn register(&self) -> twilight_model::application::command::Command {
        ResumeCommand::create_command().into()
    }
    async fn handle(
        &self,
        ctx: &InteractionContext,
        _data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_resume(ctx, module_ctx).await
    }
}
pub struct ResumePrefixCommand;
#[async_trait::async_trait]
impl PrefixCommand for ResumePrefixCommand {
    fn aliases(&self) -> Vec<&'static str> {
        vec!["resume", "unpause"]
    }
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_prefix_resume(ctx, module_ctx).await
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "volume", desc = "Set the player volume (0-200)")]
pub struct VolumeCommand {
    #[command(desc = "Volume level percentage")]
    pub level: i64,
}

pub struct VolumeAppCommand;
#[async_trait::async_trait]
impl AppCommand for VolumeAppCommand {
    fn name(&self) -> &'static str {
        "volume"
    }
    fn register(&self) -> twilight_model::application::command::Command {
        VolumeCommand::create_command().into()
    }
    async fn handle(
        &self,
        ctx: &InteractionContext,
        data: &CommandData,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        let cmd = VolumeCommand::from_interaction(data.clone().into())
            .map_err(|e| HarmonyError::Internal(e.to_string()))?;
        handle_volume(ctx, cmd, module_ctx).await
    }
}
pub struct VolumePrefixCommand;
#[async_trait::async_trait]
impl PrefixCommand for VolumePrefixCommand {
    fn aliases(&self) -> Vec<&'static str> {
        vec!["volume", "vol", "v"]
    }
    async fn handle(
        &self,
        ctx: &PrefixContext,
        module_ctx: &ModuleContext,
    ) -> Result<(), HarmonyError> {
        handle_prefix_volume(ctx, module_ctx).await
    }
}

async fn send_music_command(
    cmd: MusicCommand,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let payload = serde_json::to_string(&cmd).unwrap_or_default();
    let mut redis_conn = module_ctx.cache.clone();
    let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;
    Ok(())
}

pub async fn handle_stop(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Stop { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏹️ Stopped playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_skip(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Skip { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏭️ Skipped the current track.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_pause(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Pause { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("⏸️ Paused playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_resume(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    send_music_command(MusicCommand::Resume { guild_id: guild_id.to_string() }, module_ctx).await?;

    let embed = EmbedBuilder::new()
        .description("▶️ Resumed playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_volume(
    interaction_ctx: &InteractionContext,
    cmd: VolumeCommand,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let _guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let _vol = cmd.level;

    let embed = EmbedBuilder::new()
        .description("🔊 Volume control over IPC is not yet supported.")
        .color(0xFF0000)
        .build();
    let _ = interaction_ctx.reply_embed(embed, module_ctx).await;

    Ok(())
}

pub async fn handle_prefix_stop(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Stop { guild_id: ctx.guild_id.to_string() }, module_ctx)
        .await?;

    let embed = EmbedBuilder::new()
        .description("⏹️ Stopped playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_skip(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Skip { guild_id: ctx.guild_id.to_string() }, module_ctx)
        .await?;

    let embed = EmbedBuilder::new()
        .description("⏭️ Skipped the current track.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_pause(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Pause { guild_id: ctx.guild_id.to_string() }, module_ctx)
        .await?;

    let embed = EmbedBuilder::new()
        .description("⏸️ Paused playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_resume(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    send_music_command(MusicCommand::Resume { guild_id: ctx.guild_id.to_string() }, module_ctx)
        .await?;

    let embed = EmbedBuilder::new()
        .description("▶️ Resumed playback.")
        .color(module_ctx.embed_color)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}

pub async fn handle_prefix_volume(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let embed = EmbedBuilder::new()
        .description("🔊 Volume control over IPC is not yet supported.")
        .color(0xFF0000)
        .build();
    let _ = ctx.reply_embed(embed, module_ctx).await;
    Ok(())
}
