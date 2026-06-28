use railway_cache::ratelimit::RateLimiter;
use railway_common::error::RailwayError;
use railway_common::event::RailwayEvent;
use railway_common::module::ModuleContext;
use railway_database::repository::antinuke_repository::AntinukeRepository;
use twilight_model::gateway::event::Event;

pub struct Detector {}

impl Detector {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn analyze(
        &self,
        event: &RailwayEvent,
        ctx: &ModuleContext,
    ) -> Result<bool, RailwayError> {
        if let RailwayEvent::Discord(discord_event) = event {
            match **discord_event {
                Event::BanAdd(ref ban) => {
                    let repo = AntinukeRepository::new(ctx.db.clone());
                    let guild_id = ban.guild_id.get() as i64;
                    let user_id = ban.user.id.get() as i64;

                    if let Some(config) = repo.get_config(guild_id).await? {
                        if !config.enabled {
                            return Ok(false);
                        }

                        if repo.is_whitelisted(guild_id, user_id).await? {
                            return Ok(false);
                        }

                        let key = format!("antinuke:ban:{}:{}", guild_id, user_id);
                        let mut conn = ctx
                            .cache
                            .get_multiplexed_tokio_connection()
                            .await
                            .map_err(RailwayError::Cache)?;
                        let current_count =
                            RateLimiter::check_and_increment(&mut conn, &key, 60).await?;

                        if current_count >= config.ban_threshold as i64 {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                Event::ChannelDelete(_) => Ok(false),
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}
