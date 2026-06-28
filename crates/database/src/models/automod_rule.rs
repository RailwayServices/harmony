use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct AutomodRule {
    pub id: i32,
    pub guild_id: i64,
    pub name: String,
    pub trigger_type: i16,
    pub action_type: i16,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
