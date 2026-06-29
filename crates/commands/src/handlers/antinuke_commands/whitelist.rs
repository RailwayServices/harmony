use super::AntinukeCommandHandler;
use railway_common::error::RailwayError;
use railway_common::module::ModuleContext;
use railway_database::repository::antinuke_repository::AntinukeRepository;

impl AntinukeCommandHandler {
    pub(super) async fn handle_whitelist_add(
        &self,
        guild_id: i64,
        user_id: i64,
        added_by: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        repo.add_whitelist(guild_id, user_id, added_by).await?;

        railway_antinuke::whitelist_add(guild_id as u64, user_id as u64);

        Ok(format!("✅ User <@{}> added to whitelist.", user_id))
    }

    pub(super) async fn handle_whitelist_remove(
        &self,
        guild_id: i64,
        user_id: i64,
        module_ctx: &ModuleContext,
    ) -> Result<String, RailwayError> {
        let repo = AntinukeRepository::new(module_ctx.db.clone());
        repo.remove_whitelist(guild_id, user_id).await?;

        railway_antinuke::whitelist_remove(guild_id as u64, user_id as u64);

        Ok(format!("✅ User <@{}> removed from whitelist.", user_id))
    }
}
