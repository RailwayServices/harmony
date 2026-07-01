use railway_common::error::RailwayError;
use std::sync::Arc;
use tokio::sync::OnceCell;
use twilight_http::Client as HttpClient;
use twilight_model::application::command::CommandType;
use twilight_model::guild::Permissions;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::id::Id;
use twilight_util::builder::command::{
    CommandBuilder, IntegerBuilder, StringBuilder, SubCommandBuilder, SubCommandGroupBuilder,
    UserBuilder,
};

static APP_ID: OnceCell<Id<ApplicationMarker>> = OnceCell::const_new();

pub async fn register_global_commands(http: Arc<HttpClient>) -> Result<(), RailwayError> {
    let app_info = http.current_user_application().await?.model().await?;
    let app_id = app_info.id;
    APP_ID.set(app_id).ok();

    let interaction_client = http.interaction(app_id);

    let antinuke_cmd = CommandBuilder::new(
        "antinuke",
        "Configure the Railway AntiNuke engine",
        CommandType::ChatInput,
    )
    .default_member_permissions(Permissions::MANAGE_GUILD)
    .option(SubCommandBuilder::new("enable", "Enable antinuke for this server").build())
    .option(SubCommandBuilder::new("disable", "Disable antinuke for this server").build())
    .option(
        SubCommandBuilder::new("punishment", "Set the default punishment for destructive actions")
            .option(StringBuilder::new("action", "The action to take").required(true).choices(
                vec![
                    ("Ban", "ban"),
                    ("Kick", "kick"),
                    ("Strip Roles", "strip_roles"),
                    ("Timeout (30m)", "timeout"),
                ],
            ))
            .build(),
    )
    .option(
        SubCommandBuilder::new("limit", "Set granular limits for a specific action")
            .option(
                StringBuilder::new("action", "The action to set a limit for")
                    .required(true)
                    .choices(vec![
                        ("Bans", "BAN_ADD"),
                        ("Kicks", "MEMBER_KICK"),
                        ("Channel Deletes", "CHANNEL_DELETE"),
                        ("Role Deletes", "ROLE_DELETE"),
                        ("Bot Adds", "BOT_ADD"),
                        ("Mass Mentions", "EVERYONE_PING"),
                    ]),
            )
            .option(
                IntegerBuilder::new("threshold", "Number of actions allowed (0 = instant)")
                    .required(true)
                    .min_value(0)
                    .max_value(20),
            )
            .option(
                IntegerBuilder::new("window_secs", "Time window in seconds")
                    .required(true)
                    .min_value(1)
                    .max_value(300),
            )
            .option(
                StringBuilder::new("punishment", "The punishment to apply").required(true).choices(
                    vec![
                        ("Ban", "BAN"),
                        ("Kick", "KICK"),
                        ("Strip Roles", "STRIP_ROLES"),
                        ("Timeout (30m)", "TIMEOUT"),
                    ],
                ),
            )
            .build(),
    )
    .option(SubCommandBuilder::new("settings", "View current antinuke settings").build())
    .option(
        SubCommandGroupBuilder::new("whitelist", "Manage users exempt from antinuke")
            .subcommands(vec![
                SubCommandBuilder::new("add", "Add a user to the whitelist")
                    .option(UserBuilder::new("user", "The user to whitelist").required(true)),
                SubCommandBuilder::new("remove", "Remove a user from the whitelist")
                    .option(UserBuilder::new("user", "The user to remove").required(true)),
            ])
            .build(),
    )
    .build();

    let automod_cmd = CommandBuilder::new(
        "automod",
        "Configure the Railway AutoMod engine",
        CommandType::ChatInput,
    )
    .default_member_permissions(Permissions::MANAGE_GUILD)
    .option(
        SubCommandBuilder::new("enable", "Enable an automod filter")
            .option(StringBuilder::new("filter", "The filter to enable").required(true).choices(
                vec![
                    ("Spam Filter", "spam"),
                    ("Anti-Link (Invites)", "antilink"),
                    ("Ghost Ping", "ghostping"),
                ],
            ))
            .build(),
    )
    .option(
        SubCommandBuilder::new("disable", "Disable an automod filter")
            .option(StringBuilder::new("filter", "The filter to disable").required(true).choices(
                vec![
                    ("Spam Filter", "spam"),
                    ("Anti-Link (Invites)", "antilink"),
                    ("Ghost Ping", "ghostping"),
                ],
            ))
            .build(),
    )
    .option(
        SubCommandBuilder::new("punishment", "Set the punishment for a filter")
            .option(StringBuilder::new("filter", "The filter to configure").required(true).choices(
                vec![
                    ("Spam Filter", "spam"),
                    ("Anti-Link (Invites)", "antilink"),
                    ("Ghost Ping", "ghostping"),
                ],
            ))
            .option(StringBuilder::new("action", "The action to take").required(true).choices(
                vec![
                    ("Delete Message", "delete"),
                    ("Timeout (5m)", "timeout"),
                    ("Delete & Timeout", "delete_and_timeout"),
                ],
            ))
            .build(),
    )
    .option(SubCommandBuilder::new("settings", "View current AutoMod settings").build())
    .build();

    let commands = vec![antinuke_cmd, automod_cmd];

    interaction_client.set_global_commands(&commands).await?;

    tracing::info!("Registered global application commands successfully");
    Ok(())
}
