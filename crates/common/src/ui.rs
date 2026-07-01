use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};
use twilight_model::channel::message::Embed;
use twilight_util::builder::embed::EmbedBuilder;

pub fn build_stylish_embed(title: &str, description: &str, color: u32) -> Embed {
    EmbedBuilder::new().title(title).description(description).color(color).build()
}

pub fn build_support_action_row() -> Component {
    let support_btn = Button {
        custom_id: None,
        disabled: false,
        emoji: None,
        label: Some("Support Server".to_string()),
        style: ButtonStyle::Link,
        url: Some("https://discord.gg/fhdgshs".to_string()),
        sku_id: None,
        id: None,
    };

    Component::ActionRow(ActionRow { components: vec![Component::Button(support_btn)], id: None })
}

pub fn build_antinuke_log_buttons(user_id: u64) -> Component {
    let whitelist_btn = Button {
        custom_id: Some(format!("antinuke_wl:{}", user_id)),
        disabled: false,
        emoji: None,
        label: Some("Whitelist User".to_string()),
        style: ButtonStyle::Secondary,
        url: None,
        sku_id: None,
        id: None,
    };

    let unban_btn = Button {
        custom_id: Some(format!("antinuke_unban:{}", user_id)),
        disabled: false,
        emoji: None,
        label: Some("Unban User".to_string()),
        style: ButtonStyle::Danger,
        url: None,
        sku_id: None,
        id: None,
    };

    Component::ActionRow(ActionRow {
        components: vec![Component::Button(whitelist_btn), Component::Button(unban_btn)],
        id: None,
    })
}

pub fn build_antinuke_settings_buttons(enabled: bool) -> Component {
    let toggle_btn = Button {
        custom_id: Some("antinuke_toggle".to_string()),
        disabled: false,
        emoji: None,
        label: Some(if enabled {
            "Disable AntiNuke".to_string()
        } else {
            "Enable AntiNuke".to_string()
        }),
        style: if enabled { ButtonStyle::Danger } else { ButtonStyle::Success },
        url: None,
        sku_id: None,
        id: None,
    };

    Component::ActionRow(ActionRow { components: vec![Component::Button(toggle_btn)], id: None })
}

pub fn build_automod_settings_buttons(spam: bool, antilink: bool, ghostping: bool) -> Component {
    let spam_btn = Button {
        custom_id: Some("automod_toggle:spam".to_string()),
        disabled: false,
        emoji: None,
        label: Some(if spam { "Spam (Active)".to_string() } else { "Spam (Disabled)".to_string() }),
        style: if spam { ButtonStyle::Success } else { ButtonStyle::Danger },
        url: None,
        sku_id: None,
        id: None,
    };

    let antilink_btn = Button {
        custom_id: Some("automod_toggle:antilink".to_string()),
        disabled: false,
        emoji: None,
        label: Some(if antilink {
            "Anti-Link (Active)".to_string()
        } else {
            "Anti-Link (Disabled)".to_string()
        }),
        style: if antilink { ButtonStyle::Success } else { ButtonStyle::Danger },
        url: None,
        sku_id: None,
        id: None,
    };

    let ghostping_btn = Button {
        custom_id: Some("automod_toggle:ghostping".to_string()),
        disabled: false,
        emoji: None,
        label: Some(if ghostping {
            "GhostPing (Active)".to_string()
        } else {
            "GhostPing (Disabled)".to_string()
        }),
        style: if ghostping { ButtonStyle::Success } else { ButtonStyle::Danger },
        url: None,
        sku_id: None,
        id: None,
    };

    Component::ActionRow(ActionRow {
        components: vec![
            Component::Button(spam_btn),
            Component::Button(antilink_btn),
            Component::Button(ghostping_btn),
        ],
        id: None,
    })
}
