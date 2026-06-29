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
