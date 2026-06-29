use super::AntinukeCommandHandler;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::repository::antinuke_repository::AntinukeRepository;

impl AntinukeCommandHandler {
    pub(super) async fn handle_settings(
        &self,
        guild_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        let config = repo.get_config(guild_id).await?;
        let modules = repo.get_module_configs(guild_id).await?;
        let wl_count = repo.get_whitelist_count(guild_id).await?;

        if let Some(c) = config {
            let status = if c.enabled { "🟢 Enabled" } else { "🔴 Disabled" };
            let log_ch = c
                .log_channel_id
                .map(|id| format!("<#{}>", id))
                .unwrap_or_else(|| "None".to_string());

            let mut mods_str = String::new();
            for m in modules {
                mods_str.push_str(&format!(
                    "- {}: {}/{}s ({})\n",
                    m.action_type, m.threshold, m.window_secs, m.punishment
                ));
            }
            if mods_str.is_empty() {
                mods_str = "No specific limits configured.".to_string();
            }

            Ok(format!("**AntiNuke Settings**\nStatus: {}\nLog Channel: {}\nWhitelisted Users: {}\n\n**Limits:**\n{}", status, log_ch, wl_count, mods_str))
        } else {
            Ok(format!(
                "**AntiNuke Settings**\nStatus: 🔴 Disabled\nWhitelisted Users: {}",
                wl_count
            ))
        }
    }
}
