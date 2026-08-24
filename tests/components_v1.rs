//! Round-trip suite for message components v1: action rows, buttons,
//! select menus.

use std::borrow::Cow;

use serde_json::json;
use serenity::builder::CreateActionRow;
use serenity::builder::CreateButton;
use serenity::builder::CreateSelectMenu;
use serenity::builder::CreateSelectMenuKind;
use serenity::builder::CreateSelectMenuOption;
use serenity::model::application::ButtonStyle;
use serenity::model::channel::ChannelType;
use serenity::model::channel::ReactionType;
use serenity::model::id::EmojiId;
use serenity::model::id::GenericChannelId;
use serenity::model::id::RoleId;
use serenity::model::id::SkuId;
use serenity::model::id::UserId;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use helpers::to_value;
use pwr_ext::prelude::*;

// ---- CreateButton ----

#[test]
fn button_with_all_fields_set_roundtrips() {
    let button = CreateButton::new("verify:panel:42")
        .label("Verify account")
        .style(ButtonStyle::Success)
        .emoji(ReactionType::Custom {
            animated: false,
            id: EmojiId::new(936_926_320_553_408_522),
            name: Some("tick_green".to_string().try_into().expect("non-empty name")),
        })
        .disabled(true);

    assert_roundtrip::<CreateButtonDe<'static>, _>(&button);
}

#[test]
fn minimal_custom_button_roundtrips() {
    let button = CreateButton::new("delete:msg:17");

    assert_roundtrip::<CreateButtonDe<'static>, _>(&button);
}

#[test]
fn link_button_roundtrips() {
    let button =
        CreateButton::new_link("https://status.example.com/incident/4021").label("Status page");

    assert_roundtrip::<CreateButtonDe<'static>, _>(&button);
}

#[test]
fn premium_button_roundtrips() {
    let button = CreateButton::new_premium(SkuId::new(1_234_567_890_123_456_789));

    assert_roundtrip::<CreateButtonDe<'static>, _>(&button);
}

#[test]
fn unicode_emoji_button_roundtrips() {
    let button = CreateButton::new("react:star")
        .label("Star")
        .emoji(ReactionType::Unicode(
            "⭐".to_string().try_into().expect("non-empty"),
        ));

    assert_roundtrip::<CreateButtonDe<'static>, _>(&button);
}

#[test]
fn every_button_style_value_survives_the_roundtrip() {
    // 1..=4 are the named styles; 5 and 6 are Link/Premium, represented
    // upstream as Unknown(u8); 9 and 255 are values only Unknown can carry.
    let styles = [
        ButtonStyle::Primary,
        ButtonStyle::Secondary,
        ButtonStyle::Success,
        ButtonStyle::Danger,
        ButtonStyle::Unknown(5),
        ButtonStyle::Unknown(6),
        ButtonStyle::Unknown(9),
        ButtonStyle::Unknown(255),
    ];

    for style in styles {
        let button = CreateButton::new("style:probe").style(style);
        let original = to_value(&button);
        let rebuilt = rebuild::<CreateButtonDe<'static>, CreateButton<'static>>(&original);

        assert_value_eq(&original, &rebuilt);
    }
}

#[test]
fn button_without_disabled_field_defaults_to_false() {
    // D7: upstream always serializes `disabled`, so input omitting it
    // canonically gains `"disabled": false`.
    let payload = json!({
        "type": 2,
        "style": 1,
        "custom_id": "kick:confirm",
        "label": "Kick",
    });
    let expected = json!({
        "type": 2,
        "style": 1,
        "custom_id": "kick:confirm",
        "label": "Kick",
        "disabled": false,
    });

    let rebuilt = rebuild::<CreateButtonDe<'static>, CreateButton<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn disabled_false_and_true_are_distinguishable_after_the_roundtrip() {
    let enabled = json!({
        "type": 2, "style": 2, "custom_id": "toggle:lamp", "disabled": false,
    });
    let disabled = json!({
        "type": 2, "style": 2, "custom_id": "toggle:lamp", "disabled": true,
    });

    let rebuilt_enabled = rebuild::<CreateButtonDe<'static>, CreateButton<'static>>(&enabled);
    let rebuilt_disabled = rebuild::<CreateButtonDe<'static>, CreateButton<'static>>(&disabled);

    assert_ne!(rebuilt_enabled, rebuilt_disabled);
    assert_value_eq(&enabled, &rebuilt_enabled);
    assert_value_eq(&disabled, &rebuilt_disabled);
}

#[test]
fn button_missing_numeric_type_tag_errors() {
    let payload = json!({ "style": 1, "custom_id": "no:type:tag" });

    let result: Result<CreateButtonDe<'static>, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "a button object without \"type\" must be rejected"
    );
}

// ---- CreateActionRow ----

#[test]
fn action_row_of_buttons_roundtrips() {
    let row = CreateActionRow::buttons(vec![
        CreateButton::new("ack:4021")
            .label("Acknowledge")
            .style(ButtonStyle::Success),
        CreateButton::new_link("https://status.example.com").label("Status page"),
        CreateButton::new("resolve:4021")
            .label("Resolve")
            .style(ButtonStyle::Danger)
            .disabled(true),
    ]);

    assert_roundtrip::<CreateActionRowDe<'static>, _>(&row);
}

#[test]
fn empty_button_action_row_roundtrips() {
    let row = CreateActionRow::buttons(Vec::<CreateButton>::new());

    assert_roundtrip::<CreateActionRowDe<'static>, _>(&row);
}

#[test]
fn action_row_with_select_menu_roundtrips() {
    let row = CreateActionRow::select_menu(
        CreateSelectMenu::new(
            "role:pick",
            CreateSelectMenuKind::String {
                options: Cow::Owned(vec![
                    CreateSelectMenuOption::new("Rust", "lang:rust").default_selection(true),
                    CreateSelectMenuOption::new("TypeScript", "lang:ts"),
                ]),
            },
        )
        .placeholder("Pick your language")
        .min_values(1)
        .max_values(2),
    );

    assert_roundtrip::<CreateActionRowDe<'static>, _>(&row);
}

#[test]
fn action_row_missing_type_tag_errors() {
    let payload = json!({ "components": [{ "type": 2, "style": 1, "custom_id": "x:y" }] });

    let result: Result<CreateActionRowDe<'static>, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "an action row without \"type\" must be rejected"
    );
}

#[test]
fn action_row_with_wrong_type_tag_errors() {
    let payload = json!({
        "type": 9,
        "components": [{ "type": 10, "content": "not an action row" }],
    });

    let result: Result<CreateActionRowDe<'static>, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "\"type\": 9 must not parse as an action row"
    );
}

#[test]
fn action_row_mixing_select_menu_and_buttons_errors() {
    // A select-menu row serializes as exactly one component; siblings make
    // the row unrepresentable.
    let payload = json!({
        "type": 1,
        "components": [
            { "type": 3, "custom_id": "pick:one", "options": [{ "label": "A", "value": "a" }] },
            { "type": 2, "style": 1, "custom_id": "btn:1" },
        ],
    });

    let result: Result<CreateActionRowDe<'static>, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "buttons may not follow a select menu in one row"
    );
}

// ---- CreateSelectMenu per kind ----

#[test]
fn user_select_menu_with_defaults_roundtrips() {
    let menu = CreateSelectMenu::new(
        "user:assign",
        CreateSelectMenuKind::User {
            default_users: Some(Cow::Owned(vec![
                UserId::new(8_077_972_345_678_905_346),
                UserId::new(2),
            ])),
        },
    );

    assert_roundtrip::<CreateActionRowDe<'static>, _>(&CreateActionRow::select_menu(menu));
}

#[test]
fn role_select_menu_with_default_roundtrips() {
    let menu = CreateSelectMenu::new(
        "role:grant",
        CreateSelectMenuKind::Role {
            default_roles: Some(Cow::Owned(vec![RoleId::new(777)])),
        },
    );

    assert_roundtrip::<CreateActionRowDe<'static>, _>(&CreateActionRow::select_menu(menu));
}

#[test]
fn channel_select_menu_with_channel_types_and_defaults_roundtrips() {
    let menu = CreateSelectMenu::new(
        "channel:move",
        CreateSelectMenuKind::Channel {
            channel_types: Some(Cow::Owned(vec![ChannelType::Text, ChannelType::Voice])),
            default_channels: Some(Cow::Owned(vec![GenericChannelId::new(42)])),
        },
    );

    assert_roundtrip::<CreateActionRowDe<'static>, _>(&CreateActionRow::select_menu(menu));
}

#[test]
fn mentionable_select_menu_with_users_and_roles_roundtrips() {
    let menu = CreateSelectMenu::new(
        "mention:pick",
        CreateSelectMenuKind::Mentionable {
            default_users: Some(Cow::Owned(vec![UserId::new(100)])),
            default_roles: Some(Cow::Owned(vec![RoleId::new(200)])),
        },
    );

    assert_roundtrip::<CreateActionRowDe<'static>, _>(&CreateActionRow::select_menu(menu));
}

#[test]
fn select_menu_toggles_are_optional_and_preserved() {
    // required/disabled omitted vs present must stay distinguishable.
    let bare = json!({
        "type": 1,
        "components": [{ "type": 3, "custom_id": "bare:menu", "options": [] }],
    });
    let toggled = json!({
        "type": 1,
        "components": [{
            "type": 3,
            "custom_id": "toggled:menu",
            "options": [],
            "min_values": 0,
            "max_values": 25,
            "required": false,
            "disabled": true,
        }],
    });

    let rebuilt_bare = rebuild::<CreateActionRowDe<'static>, CreateActionRow<'static>>(&bare);
    let rebuilt_toggled = rebuild::<CreateActionRowDe<'static>, CreateActionRow<'static>>(&toggled);

    assert_ne!(rebuilt_bare, rebuilt_toggled);
    assert_value_eq(&bare, &rebuilt_bare);
    assert_value_eq(&toggled, &rebuilt_toggled);
}

#[test]
fn string_select_option_full_shape_roundtrips() {
    let payload = json!({
        "type": 1,
        "components": [{
            "type": 3,
            "custom_id": "fav:framework",
            "options": [{
                "label": "Axum",
                "value": "axum",
                "description": "Ergonomic web framework",
                "emoji": { "name": "🦀" },
                "default": true,
            }],
        }],
    });

    let rebuilt = rebuild::<CreateActionRowDe<'static>, CreateActionRow<'static>>(&payload);

    assert_value_eq(&payload, &rebuilt);
}

#[test]
fn every_select_menu_kind_number_is_dispatched() {
    // 3 String, 5 User, 6 Role, 7 Mentionable, 8 Channel.
    let cases = [
        json!({ "type": 1, "components": [{ "type": 3, "custom_id": "probe:menu", "options": [] }] }),
        json!({ "type": 1, "components": [{ "type": 5, "custom_id": "probe:menu" }] }),
        json!({ "type": 1, "components": [{ "type": 6, "custom_id": "probe:menu" }] }),
        json!({ "type": 1, "components": [{ "type": 7, "custom_id": "probe:menu" }] }),
        json!({ "type": 1, "components": [{ "type": 8, "custom_id": "probe:menu" }] }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateActionRowDe<'static>, CreateActionRow<'static>>(&payload);

        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn select_menu_with_unknown_kind_number_errors() {
    // 4 is InputText — a real component type, but not a select-menu kind.
    let payload = json!({
        "type": 1,
        "components": [{ "type": 4, "custom_id": "text:input" }],
    });

    let result: Result<CreateActionRowDe<'static>, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "input text inside an action row must be rejected"
    );
}
