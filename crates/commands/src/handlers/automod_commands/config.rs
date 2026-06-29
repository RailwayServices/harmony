use super::AutomodCommandHandler;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::models::automod_rule::{ActionType, TriggerType};
use railway_database::repository::automod_repository::AutomodRepository;

impl AutomodCommandHandler {
    fn parse_filter(filter: &str) -> Option<TriggerType> {
        match filter {
            "spam" => Some(TriggerType::Spam),
            "antilink" => Some(TriggerType::AntiLink),
            "ghostping" => Some(TriggerType::GhostPing),
            _ => None,
        }
    }

    fn parse_action(action: &str) -> Option<ActionType> {
        match action {
            "delete" => Some(ActionType::DeleteMessage),
            "timeout" => Some(ActionType::Timeout),
            "delete_and_timeout" => Some(ActionType::DeleteAndTimeout),
            _ => None,
        }
    }

    pub async fn handle_enable(
        &self,
        guild_id: i64,
        filter: String,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let trigger = match Self::parse_filter(&filter) {
            Some(t) => t,
            None => return Ok("Invalid filter type.".to_string()),
        };

        // Check if rule exists, otherwise create
        let rule = AutomodRepository::get_rule(&module_ctx.db, guild_id, trigger as i16)
            .await
            .ok()
            .flatten();

        if let Some(mut r) = rule {
            r.enabled = true;
            AutomodRepository::update_rule(&module_ctx.db, &r).await?;
        } else {
            // Default action depends on filter
            let action = match trigger {
                TriggerType::Spam => ActionType::Timeout,
                TriggerType::AntiLink => ActionType::DeleteMessage,
                TriggerType::GhostPing => ActionType::Timeout,
            };

            AutomodRepository::create_rule(
                &module_ctx.db,
                guild_id,
                &format!("AutoMod {:?}", trigger),
                trigger as i16,
                action as i16,
                true,
            )
            .await?;
        }

        Ok(format!("Successfully enabled **{:?}** filter.", trigger))
    }

    pub async fn handle_disable(
        &self,
        guild_id: i64,
        filter: String,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let trigger = match Self::parse_filter(&filter) {
            Some(t) => t,
            None => return Ok("Invalid filter type.".to_string()),
        };

        if let Some(mut rule) =
            AutomodRepository::get_rule(&module_ctx.db, guild_id, trigger as i16)
                .await
                .ok()
                .flatten()
        {
            rule.enabled = false;
            AutomodRepository::update_rule(&module_ctx.db, &rule).await?;
            Ok(format!("Successfully disabled **{:?}** filter.", trigger))
        } else {
            Ok(format!("The **{:?}** filter is already disabled.", trigger))
        }
    }

    pub async fn handle_punishment(
        &self,
        guild_id: i64,
        filter: String,
        action_str: String,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let trigger = match Self::parse_filter(&filter) {
            Some(t) => t,
            None => return Ok("Invalid filter type.".to_string()),
        };

        let action = match Self::parse_action(&action_str) {
            Some(a) => a,
            None => return Ok("Invalid action type.".to_string()),
        };

        if let Some(mut rule) =
            AutomodRepository::get_rule(&module_ctx.db, guild_id, trigger as i16)
                .await
                .ok()
                .flatten()
        {
            rule.action_type = action as i16;
            AutomodRepository::update_rule(&module_ctx.db, &rule).await?;
            Ok(format!("Updated punishment for **{:?}** to **{:?}**.", trigger, action))
        } else {
            Ok(format!("The **{:?}** filter must be enabled first before configuring punishment.", trigger))
        }
    }
}
