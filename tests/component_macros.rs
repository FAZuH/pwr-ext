//! Parity tests for the `component!` standalone emission macro.
//!
//! `component!` emits the bare serenity builder for one element. Each test
//! compares the macro output against a hand-built oracle (serde_json
//! comparison), and verifies that wrapping the emitted builder in the
//! appropriate enum variant serializes identically to the same element built
//! via `view!`.

use pwr_ext::component;
use pwr_ext::view;
use pwr_ext::view_support::CreateActionRow;
use pwr_ext::view_support::CreateButton;
use pwr_ext::view_support::CreateComponent;
use pwr_ext::view_support::CreateContainer;
use pwr_ext::view_support::CreateContainerComponent;
use pwr_ext::view_support::CreateMessage;
use pwr_ext::view_support::CreateSeparator;
use pwr_ext::view_support::CreateTextDisplay;
use pwr_ext::view_support::MessageFlags;
use serde_json::Value;

fn to_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

// ── action_row ─────────────────────────────────────────────────────────────

#[test]
fn action_row_matches_hand_built() {
    let via_macro = component! {
        action_row {
            button { custom_id: "b1", label: "B1" }
            button { custom_id: "b2", label: "B2" }
        }
    };
    let expected = CreateActionRow::buttons(vec![
        CreateButton::new("b1").label("B1"),
        CreateButton::new("b2").label("B2"),
    ]);
    assert_eq!(to_value(&via_macro), to_value(&expected));
}

#[test]
fn action_row_serializes_identically_to_view() {
    let via_view = view! {
        components_v2 {
            action_row {
                button { custom_id: "b1", label: "B1" }
            }
        }
    };
    let bare = component! {
        action_row {
            button { custom_id: "b1", label: "B1" }
        }
    };
    let via_component = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::ActionRow(bare)]);
    assert_eq!(to_value(&via_view), to_value(&via_component));
}

// ── text_display ───────────────────────────────────────────────────────────

#[test]
fn text_display_matches_hand_built() {
    let via_macro = component! { text_display { "hello" } };
    let expected = CreateTextDisplay::new("hello");
    assert_eq!(to_value(&via_macro), to_value(&expected));
}

#[test]
fn text_display_serializes_identically_to_view() {
    let via_view = view! {
        components_v2 {
            text_display { "hello" }
        }
    };
    let bare = component! { text_display { "hello" } };
    let via_component = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::TextDisplay(bare)]);
    assert_eq!(to_value(&via_view), to_value(&via_component));
}

// ── container ──────────────────────────────────────────────────────────────

#[test]
fn container_matches_hand_built() {
    let via_macro = component! {
        container {
            text_display { "a" }
            separator {}
        }
    };
    let expected = CreateContainer::new(vec![
        CreateContainerComponent::TextDisplay(CreateTextDisplay::new("a")),
        CreateContainerComponent::Separator(CreateSeparator::new()),
    ]);
    assert_eq!(to_value(&via_macro), to_value(&expected));
}

#[test]
fn container_serializes_identically_to_view() {
    let via_view = view! {
        components_v2 {
            container {
                text_display { "a" }
            }
        }
    };
    let bare = component! {
        container {
            text_display { "a" }
        }
    };
    let via_component = CreateMessage::new()
        .flags(MessageFlags::from_bits_truncate(1 << 15))
        .components(vec![CreateComponent::Container(bare)]);
    assert_eq!(to_value(&via_view), to_value(&via_component));
}
