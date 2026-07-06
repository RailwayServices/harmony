use super::AutomodCommandHandler;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;

impl AutomodCommandHandler {
    pub async fn handle_enable(
        &self,
        guild_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let client = module_ctx.discord.clone();
        let guild_marker = twilight_model::id::Id::new(guild_id as u64);

        use twilight_model::guild::auto_moderation::{
            AutoModerationEventType, AutoModerationKeywordPresetType,
        };

        let words = crate::handlers::automod_commands::words::ENGWORDLIST;

        // Rule 1: Keyword Filter
        let _ = client
            .create_auto_moderation_rule(
                guild_marker,
                "Prismo AutoMod Rule 1",
                AutoModerationEventType::MessageSend,
            )
            .enabled(true)
            .action_block_message()
            .with_keyword(words, &[], &[])
            .await;

        // Rule 2: Mention Spam
        let _ = client
            .create_auto_moderation_rule(
                guild_marker,
                "Prismo AutoMod Rule 2",
                AutoModerationEventType::MessageSend,
            )
            .enabled(true)
            .action_block_message()
            .with_mention_spam(5)
            .await;

        // Rule 3: Keyword Preset
        let presets = [
            AutoModerationKeywordPresetType::Profanity,
            AutoModerationKeywordPresetType::SexualContent,
            AutoModerationKeywordPresetType::Slurs,
        ];
        let _ = client
            .create_auto_moderation_rule(
                guild_marker,
                "Prismo AutoMod Rule 3",
                AutoModerationEventType::MessageSend,
            )
            .enabled(true)
            .action_block_message()
            .with_keyword_preset(&presets, &[])
            .await;

        // Rule 4: Spam
        let _ = client
            .create_auto_moderation_rule(
                guild_marker,
                "Prismo AutoMod Rule 4",
                AutoModerationEventType::MessageSend,
            )
            .enabled(true)
            .action_block_message()
            .with_spam()
            .await;

        Ok("✅ AutoMod has been **enabled** in this server! All Native rules created.".to_string())
    }

    pub async fn handle_disable(
        &self,
        guild_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let client = module_ctx.discord.clone();
        let guild_marker = twilight_model::id::Id::new(guild_id as u64);

        if let Ok(response) = client.auto_moderation_rules(guild_marker).await {
            if let Ok(rules) = response.model().await {
                for rule in rules {
                    let _ = client.delete_auto_moderation_rule(guild_marker, rule.id).await;
                }
            }
        }

        Ok("✅ AutoMod has been **disabled** in this server! All Native rules deleted.".to_string())
    }
}
