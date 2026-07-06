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
        SubCommandGroupBuilder::new("set", "Set configurations for antinuke")
            .subcommands(vec![SubCommandBuilder::new(
                "limit",
                "Set granular limits for a specific action",
            )
            .option(
                StringBuilder::new("action", "The action to set a limit for")
                    .required(true)
                    .choices(vec![
                        ("Banning Members", "BAN_ADD"),
                        ("Kicking Members", "MEMBER_KICK"),
                        ("Deleting Roles", "ROLE_DELETE"),
                        ("Creating Roles", "ROLE_CREATE"),
                        ("Deleting Channels", "CHANNEL_DELETE"),
                        ("Creating Channels", "CHANNEL_CREATE"),
                    ]),
            )
            .option(
                IntegerBuilder::new("limit", "Number of actions allowed (0 = instant punishment)")
                    .required(true)
                    .min_value(0)
                    .max_value(20),
            )])
            .build(),
    )
    .option(
        SubCommandBuilder::new("punishment", "Set punishment for an action")
            .option(
                StringBuilder::new("action", "The action to set punishment for")
                    .required(true)
                    .choices(vec![
                        ("Banning Members", "BAN_ADD"),
                        ("Kicking Members", "MEMBER_KICK"),
                        ("Deleting Roles", "ROLE_DELETE"),
                        ("Creating Roles", "ROLE_CREATE"),
                        ("Deleting Channels", "CHANNEL_DELETE"),
                        ("Creating Channels", "CHANNEL_CREATE"),
                    ]),
            )
            .option(
                StringBuilder::new("punishment", "The punishment to apply").required(true).choices(
                    vec![("Ban", "BAN"), ("Kick", "KICK"), ("Strip Roles", "STRIP_ROLES")],
                ),
            )
            .build(),
    )
    .option(SubCommandBuilder::new("whitelisted", "View how many users are whitelisted").build())
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
    .option(SubCommandBuilder::new("enable", "Enable AutoModeration").build())
    .option(SubCommandBuilder::new("disable", "Disable AutoModeration").build())
    .option(SubCommandBuilder::new("settings", "View current AutoMod settings").build())
    .build();

    let commands = vec![antinuke_cmd, automod_cmd];

    interaction_client.set_global_commands(&commands).await?;

    tracing::info!("Registered global application commands successfully");
    Ok(())
}
