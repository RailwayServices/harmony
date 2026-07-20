use crate::core::interaction::InteractionContext;
use crate::core::prefix::PrefixContext;
use harmony_common::error::HarmonyError;
use harmony_common::module::ModuleContext;
use harmony_common::music_ipc::{MusicCommand, MusicResponse};
use harmony_modules::MUSIC_RESPONSES;
use lavende::LoadResult;
use tokio::time::{Duration, timeout};
use twilight_util::builder::embed::{EmbedBuilder, ImageSource};

pub async fn handle(
    interaction_ctx: &InteractionContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = interaction_ctx
        .guild_id
        .ok_or_else(|| HarmonyError::Internal("Command must be run in a guild".to_string()))?;

    let user_id = interaction_ctx
        .user_id
        .ok_or_else(|| HarmonyError::Internal("Could not extract user id".to_string()))?;

    let query = interaction_ctx.extract_string_option("query").unwrap_or_default();

    if query.is_empty() {
        let embed = EmbedBuilder::new()
            .description("❌ Please provide a search query or URL.")
            .color(0xFF0000)
            .build();
        return interaction_ctx.reply_embed(embed, module_ctx).await;
    }

    if query.len() > 500 {
        let embed = EmbedBuilder::new()
            .description("❌ Query too long! Maximum 500 characters.")
            .color(0xFF0000)
            .build();
        return interaction_ctx.reply_embed(embed, module_ctx).await;
    }

    if query.starts_with("http") {
        if !query.starts_with("https://") && !query.starts_with("http://") {
            let embed =
                EmbedBuilder::new().description("❌ Invalid URL format!").color(0xFF0000).build();
            return interaction_ctx.reply_embed(embed, module_ctx).await;
        }
        if query.contains("localhost") || query.contains("127.0.0.1") || query.contains("0.0.0.0") {
            let embed = EmbedBuilder::new()
                .description("❌ Cannot play from local URLs!")
                .color(0xFF0000)
                .build();
            return interaction_ctx.reply_embed(embed, module_ctx).await;
        }
    }

    let channel_id = {
        let mut redis_conn = module_ctx.cache.clone();
        use redis::AsyncCommands;
        let cid: Option<String> =
            redis_conn.hget("harmony:voice_states", user_id.to_string()).await.unwrap_or(None);

        match cid {
            Some(id) => id,
            None => {
                let embed = EmbedBuilder::new()
                    .description("❌ You must be in a voice channel!")
                    .color(0xFF0000)
                    .build();
                return interaction_ctx.reply_embed(embed, module_ctx).await;
            }
        }
    };

    interaction_ctx.defer(module_ctx).await?;

    let search_query =
        if query.starts_with("http") { query.clone() } else { format!("ytsearch:{}", query) };

    let req_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();

    if let Some(map) = MUSIC_RESPONSES.get() {
        map.insert(req_id.clone(), tx);
    } else {
        return Err(HarmonyError::Internal("Music responses map not initialized".to_string()));
    }

    let text_channel_id =
        interaction_ctx.interaction.channel.as_ref().map(|c| c.id.get().to_string());

    let cmd = MusicCommand::Play {
        req_id: req_id.clone(),
        guild_id: guild_id.to_string(),
        channel_id: channel_id.to_string(),
        text_channel_id: text_channel_id.clone(),
        query: search_query,
    };

    let payload = serde_json::to_string(&cmd).unwrap_or_default();

    {
        use redis::AsyncCommands;
        let mut redis_conn = module_ctx.cache.clone();
        let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;

        if let Some(txt_id) = &text_channel_id {
            let _: Result<(), _> =
                redis_conn.hset("harmony:player:channel", guild_id.to_string(), txt_id).await;
        }
    }

    let response = match timeout(Duration::from_secs(15), rx).await {
        Ok(Ok(res)) => res,
        _ => {
            if let Some(map) = MUSIC_RESPONSES.get() {
                map.remove(&req_id);
            }
            let embed = EmbedBuilder::new()
                .description("❌ Timeout waiting for audio node.")
                .color(0xFF0000)
                .build();
            let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            return Ok(());
        }
    };

    match response {
        MusicResponse::PlayResult { result, .. } => match result {
            LoadResult::Empty {} => {
                let embed =
                    EmbedBuilder::new().description("❌ No results found.").color(0xFF0000).build();
                let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            }
            LoadResult::Track(track) => {
                let mut builder = EmbedBuilder::new()
                    .description(format!("✅ Added to queue: **{}**", track.info.title))
                    .color(module_ctx.embed_color);
                if let Some(thumb) = &track.info.artwork_url
                    && let Ok(src) = ImageSource::url(thumb.clone())
                {
                    builder = builder.thumbnail(src);
                }
                let _ = interaction_ctx.edit_embed(builder.build(), module_ctx).await;
            }
            LoadResult::Search(tracks) => {
                if let Some(track) = tracks.first() {
                    let mut builder = EmbedBuilder::new()
                        .description(format!("✅ Added to queue: **{}**", track.info.title))
                        .color(module_ctx.embed_color);
                    if let Some(thumb) = &track.info.artwork_url
                        && let Ok(src) = ImageSource::url(thumb.clone())
                    {
                        builder = builder.thumbnail(src);
                    }
                    let _ = interaction_ctx.edit_embed(builder.build(), module_ctx).await;
                }
            }
            LoadResult::Playlist(playlist) => {
                let count = playlist.tracks.len();
                let embed = EmbedBuilder::new()
                    .description(format!("📃 Added **{}** tracks from playlist", count))
                    .color(module_ctx.embed_color)
                    .build();
                let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            }
            LoadResult::Error(e) => {
                let embed = EmbedBuilder::new()
                    .description(format!("❌ Error loading track: {:?}", e.message))
                    .color(0xFF0000)
                    .build();
                let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
            }
        },
        MusicResponse::Error { message, .. } => {
            let embed = EmbedBuilder::new()
                .description(format!("❌ Search failed: {}", message))
                .color(0xFF0000)
                .build();
            let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
        }
        _ => {
            let embed = EmbedBuilder::new()
                .description("❌ Invalid response from audio node.")
                .color(0xFF0000)
                .build();
            let _ = interaction_ctx.edit_embed(embed, module_ctx).await;
        }
    }

    Ok(())
}

pub async fn handle_prefix(
    ctx: &PrefixContext,
    module_ctx: &ModuleContext,
) -> Result<(), HarmonyError> {
    let guild_id = ctx.guild_id;
    let user_id = ctx.message.author.id.get();
    let query = ctx.args.join(" ");

    if query.is_empty() {
        let embed = EmbedBuilder::new()
            .description("❌ Please provide a search query or URL.")
            .color(0xFF0000)
            .build();
        return ctx.reply_embed(embed, module_ctx).await;
    }

    if query.len() > 500 {
        let embed = EmbedBuilder::new()
            .description("❌ Query too long! Maximum 500 characters.")
            .color(0xFF0000)
            .build();
        return ctx.reply_embed(embed, module_ctx).await;
    }

    if query.starts_with("http") {
        if !query.starts_with("https://") && !query.starts_with("http://") {
            let embed =
                EmbedBuilder::new().description("❌ Invalid URL format!").color(0xFF0000).build();
            return ctx.reply_embed(embed, module_ctx).await;
        }
        if query.contains("localhost") || query.contains("127.0.0.1") || query.contains("0.0.0.0") {
            let embed = EmbedBuilder::new()
                .description("❌ Cannot play from local URLs!")
                .color(0xFF0000)
                .build();
            return ctx.reply_embed(embed, module_ctx).await;
        }
    }

    let channel_id = {
        let mut redis_conn = module_ctx.cache.clone();
        use redis::AsyncCommands;
        let cid: Option<String> =
            redis_conn.hget("harmony:voice_states", user_id.to_string()).await.unwrap_or(None);

        match cid {
            Some(id) => id,
            None => {
                let embed = EmbedBuilder::new()
                    .description("❌ You must be in a voice channel!")
                    .color(0xFF0000)
                    .build();
                return ctx.reply_embed(embed, module_ctx).await;
            }
        }
    };

    let search_query =
        if query.starts_with("http") { query.clone() } else { format!("ytsearch:{}", query) };

    let req_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();

    if let Some(map) = MUSIC_RESPONSES.get() {
        map.insert(req_id.clone(), tx);
    } else {
        return Err(HarmonyError::Internal("Music responses map not initialized".to_string()));
    }

    let text_channel_id = Some(ctx.message.channel_id.get().to_string());

    let cmd = MusicCommand::Play {
        req_id: req_id.clone(),
        guild_id: guild_id.to_string(),
        channel_id: channel_id.to_string(),
        text_channel_id: text_channel_id.clone(),
        query: search_query,
    };

    let payload = serde_json::to_string(&cmd).unwrap_or_default();

    {
        use redis::AsyncCommands;
        let mut redis_conn = module_ctx.cache.clone();
        let _: Result<(), _> = redis_conn.publish("harmony:music:requests", payload).await;

        if let Some(txt_id) = &text_channel_id {
            let _: Result<(), _> =
                redis_conn.hset("harmony:player:channel", guild_id.to_string(), txt_id).await;
        }
    }

    let response = match timeout(Duration::from_secs(15), rx).await {
        Ok(Ok(res)) => res,
        _ => {
            if let Some(map) = MUSIC_RESPONSES.get() {
                map.remove(&req_id);
            }
            let embed = EmbedBuilder::new()
                .description("❌ Timeout waiting for audio node.")
                .color(0xFF0000)
                .build();
            let _ = ctx.reply_embed(embed, module_ctx).await;
            return Ok(());
        }
    };

    match response {
        MusicResponse::PlayResult { result, .. } => match result {
            LoadResult::Empty {} => {
                let embed =
                    EmbedBuilder::new().description("❌ No results found.").color(0xFF0000).build();
                let _ = ctx.reply_embed(embed, module_ctx).await;
            }
            LoadResult::Track(track) => {
                let mut builder = EmbedBuilder::new()
                    .description(format!("✅ Added to queue: **{}**", track.info.title))
                    .color(module_ctx.embed_color);
                if let Some(thumb) = &track.info.artwork_url
                    && let Ok(src) = ImageSource::url(thumb.clone())
                {
                    builder = builder.thumbnail(src);
                }
                let _ = ctx.reply_embed(builder.build(), module_ctx).await;
            }
            LoadResult::Search(tracks) => {
                if let Some(track) = tracks.first() {
                    let mut builder = EmbedBuilder::new()
                        .description(format!("✅ Added to queue: **{}**", track.info.title))
                        .color(module_ctx.embed_color);
                    if let Some(thumb) = &track.info.artwork_url
                        && let Ok(src) = ImageSource::url(thumb.clone())
                    {
                        builder = builder.thumbnail(src);
                    }
                    let _ = ctx.reply_embed(builder.build(), module_ctx).await;
                }
            }
            LoadResult::Playlist(playlist) => {
                let count = playlist.tracks.len();
                let embed = EmbedBuilder::new()
                    .description(format!("📃 Added **{}** tracks from playlist", count))
                    .color(module_ctx.embed_color)
                    .build();
                let _ = ctx.reply_embed(embed, module_ctx).await;
            }
            LoadResult::Error(e) => {
                let embed = EmbedBuilder::new()
                    .description(format!("❌ Error loading track: {:?}", e.message))
                    .color(0xFF0000)
                    .build();
                let _ = ctx.reply_embed(embed, module_ctx).await;
            }
        },
        MusicResponse::Error { message, .. } => {
            let embed = EmbedBuilder::new()
                .description(format!("❌ Search failed: {}", message))
                .color(0xFF0000)
                .build();
            let _ = ctx.reply_embed(embed, module_ctx).await;
        }
        _ => {
            let embed = EmbedBuilder::new()
                .description("❌ Invalid response from audio node.")
                .color(0xFF0000)
                .build();
            let _ = ctx.reply_embed(embed, module_ctx).await;
        }
    }

    Ok(())
}
