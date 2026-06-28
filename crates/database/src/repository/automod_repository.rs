use crate::models::automod_rule::AutomodRule;
use sqlx::PgPool;

pub struct AutomodRepository {}

impl AutomodRepository {
    pub async fn get_rules(pool: &PgPool, guild_id: i64) -> Result<Vec<AutomodRule>, sqlx::Error> {
        let rules = sqlx::query_as!(
            AutomodRule,
            r#"
            SELECT id, guild_id, name, trigger_type, action_type, enabled, created_at, updated_at
            FROM automod_rules
            WHERE guild_id = $1 AND enabled = true
            "#,
            guild_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rules)
    }

    pub async fn insert_rule(
        pool: &PgPool,
        guild_id: i64,
        name: &str,
        trigger_type: i16,
        action_type: i16,
    ) -> Result<AutomodRule, sqlx::Error> {
        let rule = sqlx::query_as!(
            AutomodRule,
            r#"
            INSERT INTO automod_rules (guild_id, name, trigger_type, action_type)
            VALUES ($1, $2, $3, $4)
            RETURNING id, guild_id, name, trigger_type, action_type, enabled, created_at, updated_at
            "#,
            guild_id,
            name,
            trigger_type,
            action_type
        )
        .fetch_one(pool)
        .await?;

        Ok(rule)
    }
}
