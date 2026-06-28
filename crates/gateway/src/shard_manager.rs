use railway_common::error::RailwayError;
use twilight_gateway::{Intents, Shard, ShardId};

pub struct ShardManager {
    pub shard: Shard,
}

impl ShardManager {
    pub fn new(token: String) -> Result<Self, RailwayError> {
        let intents = Intents::GUILDS
            | Intents::GUILD_MESSAGES
            | Intents::MESSAGE_CONTENT
            | Intents::GUILD_MODERATION
            | Intents::GUILD_MEMBERS;

        let mut config_builder = twilight_gateway::ConfigBuilder::new(token, intents);

        config_builder = config_builder.identify_properties(
            twilight_model::gateway::payload::outgoing::identify::IdentifyProperties::new(
                "Discord VR",
                "twilight.rs",
                std::env::consts::OS,
            ),
        );

        let shard_id = ShardId::ONE;
        let activity = twilight_model::gateway::presence::Activity {
            application_id: None,
            assets: None,
            buttons: Vec::new(),
            created_at: None,
            details: None,
            emoji: None,
            flags: None,
            id: None,
            instance: None,
            kind: twilight_model::gateway::presence::ActivityType::Custom,
            name: "Custom Status".into(),
            party: None,
            secrets: None,
            state: Some(format!("🔗 /help · railway · ✦ cluster {}", shard_id.number())),
            timestamps: None,
            url: None,
        };

        let presence = twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload::new(
            vec![activity],
            false,
            None,
            twilight_model::gateway::presence::Status::Online,
        ).map_err(|e| RailwayError::Config(e.to_string()))?;

        config_builder = config_builder.presence(presence);

        let shard = Shard::with_config(shard_id, config_builder.build());

        Ok(Self { shard })
    }
}
