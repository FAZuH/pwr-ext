//! Consumer acceptance: the pwr-bot settings-hub `view_data` shape, expressed
//! with the `view!`/`component!` grammar.
//!
//! Mirrors `pwr-bot/crates/plugin/settings/src/main.rs` `view_data`: a
//! `components_v2` container holding literal text/config children plus a
//! runtime 0..N-button nav row spliced at its pinned position (after the
//! toggle select row), Option-gated on a non-empty discovery; the literal
//! About button row sits outside the container. The nav row is authored via
//! `component!` and wrapped in `CreateContainerComponent::ActionRow`
//! explicitly (decision D2). The oracle is hand-built serenity builders, not
//! derived from the macros.

use std::borrow::Cow;

use pwr_ext::component;
use pwr_ext::view;
use pwr_ext::view_support::ButtonStyle;
use pwr_ext::view_support::CreateActionRow;
use pwr_ext::view_support::CreateButton;
use pwr_ext::view_support::CreateComponent;
use pwr_ext::view_support::CreateContainer;
use pwr_ext::view_support::CreateContainerComponent;
use pwr_ext::view_support::CreateMessage;
use pwr_ext::view_support::CreateSelectMenu;
use pwr_ext::view_support::CreateSelectMenuKind;
use pwr_ext::view_support::CreateSelectMenuOption;
use pwr_ext::view_support::CreateTextDisplay;
use pwr_ext::view_support::MessageFlags;
use serde_json::Value;
use serde_json::json;

// Shape constants, verbatim from the settings hub.
const CONFIGURE_INFO: &str =
    "### Configure Feature Settings\n> 🛈  Click a button to edit settings for a specific feature.";
const TOGGLE_INFO: &str = "### Enable or Disable Features\n> 🛈  Turn features on or off. A checkmark means the feature is currently enabled.";

fn to_value(msg: CreateMessage<'static>) -> Value {
    serde_json::to_value(&msg).unwrap()
}

// ── the shape under test ───────────────────────────────────────────────────

/// The hub message: a `components_v2` container with the fixed literal
/// children (header, configure info, config button row, toggle info, toggle
/// select row) plus the Option-gated nav splice, and the About button row
/// outside the container.
fn hub_message(nav_row: Option<CreateContainerComponent<'static>>) -> CreateMessage<'static> {
    view! {
        components_v2 {
            container {
                text_display { "-# **Settings**" }
                text_display { content: CONFIGURE_INFO }
                action_row {
                    button { custom_id: "settings:config:feeds", label: "Feeds", style: ButtonStyle::Secondary }
                    button { custom_id: "settings:config:voice", label: "Voice", style: ButtonStyle::Secondary }
                    button { custom_id: "settings:config:welcome", label: "Welcome", style: ButtonStyle::Secondary }
                }
                text_display { content: TOGGLE_INFO }
                action_row {
                    select_menu {
                        custom_id: "settings:toggle",
                        select_menu_option { label: "✅ Feeds", value: "Feeds" }
                        select_menu_option { label: "⬜ Voice", value: "Voice" }
                        select_menu_option { label: "✅ Welcome", value: "Welcome" }
                    }
                }
                { nav_row }
            }
            action_row {
                button { custom_id: "settings:about", label: "🛈 About", style: ButtonStyle::Secondary }
            }
        }
    }
}

// ── hand-built oracle ──────────────────────────────────────────────────────

fn config_row() -> CreateActionRow<'static> {
    CreateActionRow::buttons(vec![
        CreateButton::new("settings:config:feeds")
            .label("Feeds")
            .style(ButtonStyle::Secondary),
        CreateButton::new("settings:config:voice")
            .label("Voice")
            .style(ButtonStyle::Secondary),
        CreateButton::new("settings:config:welcome")
            .label("Welcome")
            .style(ButtonStyle::Secondary),
    ])
}

fn toggle_row() -> CreateActionRow<'static> {
    CreateActionRow::select_menu(CreateSelectMenu::new(
        "settings:toggle",
        CreateSelectMenuKind::String {
            options: Cow::Owned(vec![
                CreateSelectMenuOption::new("✅ Feeds", "Feeds"),
                CreateSelectMenuOption::new("⬜ Voice", "Voice"),
                CreateSelectMenuOption::new("✅ Welcome", "Welcome"),
            ]),
        },
    ))
}

fn about_row() -> CreateActionRow<'static> {
    CreateActionRow::buttons(vec![
        CreateButton::new("settings:about")
            .label("🛈 About")
            .style(ButtonStyle::Secondary),
    ])
}

fn oracle(nav_row: Option<CreateActionRow<'static>>) -> CreateMessage<'static> {
    let mut children = vec![
        CreateContainerComponent::TextDisplay(CreateTextDisplay::new("-# **Settings**")),
        CreateContainerComponent::TextDisplay(CreateTextDisplay::new(CONFIGURE_INFO)),
        CreateContainerComponent::ActionRow(config_row()),
        CreateContainerComponent::TextDisplay(CreateTextDisplay::new(TOGGLE_INFO)),
        CreateContainerComponent::ActionRow(toggle_row()),
    ];
    if let Some(row) = nav_row {
        children.push(CreateContainerComponent::ActionRow(row));
    }
    CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![
            CreateComponent::Container(CreateContainer::new(children)),
            CreateComponent::ActionRow(about_row()),
        ])
}

// ── tests ──────────────────────────────────────────────────────────────────

#[test]
fn hub_with_non_empty_discovery() {
    // Runtime nav row: 0..N buttons from discovery, authored via component!
    // and wrapped explicitly (D2).
    let nav_buttons: Vec<CreateButton<'static>> = ["hello", "feeds"]
        .iter()
        .map(|target| {
            CreateButton::new(format!("settings:open:{target}")).label(format!("Open {target}"))
        })
        .collect();
    let nav_row = Some(CreateContainerComponent::ActionRow(component! {
        action_row {
            { nav_buttons }
        }
    }));

    let json = to_value(hub_message(nav_row));
    let expected = oracle(Some(CreateActionRow::buttons(vec![
        CreateButton::new("settings:open:hello").label("Open hello"),
        CreateButton::new("settings:open:feeds").label("Open feeds"),
    ])));
    assert_eq!(json, to_value(expected));

    // Nav row present at the pinned position: container children index 5,
    // immediately after the toggle select row at index 4.
    let children = &json["components"][0]["components"];
    assert_eq!(children[4]["type"], json!(1));
    assert_eq!(
        children[4]["components"][0]["custom_id"],
        json!("settings:toggle")
    );
    assert_eq!(children[5]["type"], json!(1));
    assert_eq!(
        children[5]["components"][0]["custom_id"],
        json!("settings:open:hello")
    );
    assert_eq!(
        children[5]["components"][1]["custom_id"],
        json!("settings:open:feeds")
    );
}

#[test]
fn hub_with_empty_discovery() {
    let json = to_value(hub_message(None));
    let expected = oracle(None);
    assert_eq!(json, to_value(expected));

    // Nav row absent; the five literal container children are unchanged.
    let children = &json["components"][0]["components"];
    assert_eq!(children.as_array().unwrap().len(), 5);
    assert!(
        json["components"][0]
            .to_string()
            .find("settings:open:")
            .is_none()
    );
}
