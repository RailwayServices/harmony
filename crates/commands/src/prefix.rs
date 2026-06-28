use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::Message;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

pub struct PrefixContext {
    pub message: Message,
    pub args: Vec<String>,
    pub guild_id: Id<GuildMarker>,
    pub http: Arc<HttpClient>,
}

impl PrefixContext {
    pub async fn reply(&self, content: &str) -> Result<(), RailwayError> {
        match self.http.create_message(self.message.channel_id).content(content).await {
            Ok(_) => Ok(()),
            Err(e) => Err(RailwayError::Internal(e.to_string())),
        }
    }
}

use crate::handlers::antinuke_commands::AntinukeCommandHandler;

pub struct PrefixRouter {
    antinuke_handler: AntinukeCommandHandler,
    prefix: String,
}

impl PrefixRouter {
    pub fn new(prefix: String) -> Self {
        Self { antinuke_handler: AntinukeCommandHandler::new(), prefix }
    }

    pub fn parse_prefix<'a>(&self, content: &'a str) -> Option<(&'a str, &'a str)> {
        let content = content.trim();

        let prefix = &self.prefix;
        let bot_id = railway_common::ids::get_bot_id();

        let stripped = if content.starts_with(prefix) {
            &content[prefix.len()..]
        } else if bot_id != 0 {
            let mention1 = format!("<@{}> ", bot_id);
            let mention2 = format!("<@!{}> ", bot_id);
            if content.starts_with(&mention1) {
                &content[mention1.len()..]
            } else if content.starts_with(&mention2) {
                &content[mention2.len()..]
            } else {
                return None;
            }
        } else {
            return None;
        };

        let stripped = stripped.trim_start();
        if stripped.is_empty() {
            return None;
        }

        let mut parts = stripped.splitn(2, |c: char| c.is_whitespace());
        let command_name = parts.next()?;
        let rest = parts.next().unwrap_or("").trim();

        Some((command_name, rest))
    }

    pub async fn handle_message(
        &self,
        msg: &Message,
        module_ctx: &ModuleContext,
    ) -> Result<(), RailwayError> {
        if msg.author.bot || msg.webhook_id.is_some() {
            return Ok(());
        }

        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => return Ok(()),
        };

        if let Some((command, rest)) = self.parse_prefix(&msg.content) {
            let args: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();

            let ctx = PrefixContext {
                message: msg.clone(),
                args,
                guild_id,
                http: module_ctx.discord.clone(),
            };

            match command.to_lowercase().as_str() {
                "antinuke" => {
                    self.antinuke_handler.handle_prefix(&ctx, module_ctx).await?;
                }
                "ping" => {
                    ctx.reply("Pong! 🏓").await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
