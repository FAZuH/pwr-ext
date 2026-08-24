//! Round-trip suite for the modal family: `CreateModal`, its tagged
//! component tree (`CreateModalComponent`: text displays and labels), and
//! the label-wrapped input components.

use std::borrow::Cow;

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateCheckbox;
use serenity::builder::CreateCheckboxGroup;
use serenity::builder::CreateCheckboxGroupOption;
use serenity::builder::CreateFileUpload;
use serenity::builder::CreateInputText;
use serenity::builder::CreateLabel;
use serenity::builder::CreateModal;
use serenity::builder::CreateModalComponent;
use serenity::builder::CreateRadioGroup;
use serenity::builder::CreateRadioGroupOption;
use serenity::builder::CreateSelectMenu;
use serenity::builder::CreateSelectMenuKind;
use serenity::builder::CreateSelectMenuOption;
use serenity::builder::CreateTextDisplay;
use serenity::model::application::InputTextStyle;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

fn string_select() -> CreateSelectMenu<'static> {
    CreateSelectMenu::new(
        "category",
        CreateSelectMenuKind::String {
            options: Cow::Owned(vec![
                CreateSelectMenuOption::new("Bug report", "bug").description("Something is broken"),
                CreateSelectMenuOption::new("Feature idea", "feature"),
            ]),
        },
    )
}

// ---- CreateModal ----

#[test]
fn modal_with_every_component_kind_roundtrips() {
    let modal = CreateModal::new("feedback:form:v3", "Send feedback").components(vec![
        CreateModalComponent::TextDisplay(CreateTextDisplay::new(
            "**Feedback** — tell us what worked and what didn't.",
        )),
        CreateModalComponent::Label(
            CreateLabel::input_text(
                "Summary",
                CreateInputText::new(InputTextStyle::Short, "summary").max_length(100),
            )
            .description("One line please"),
        ),
        CreateModalComponent::Label(CreateLabel::select_menu("Category", string_select())),
        CreateModalComponent::Label(CreateLabel::file_upload(
            "Screenshots",
            CreateFileUpload::new("screenshots")
                .min_values(0)
                .required(false)
                .file_types(vec![Cow::<str>::Owned("image".into())]),
        )),
        CreateModalComponent::Label(CreateLabel::radio_group(
            "Rating",
            CreateRadioGroup::new(
                "rating",
                vec![
                    CreateRadioGroupOption::new("Good", "good").default_selection(true),
                    CreateRadioGroupOption::new("Bad", "bad"),
                ],
            ),
        )),
        CreateModalComponent::Label(CreateLabel::checkbox_group(
            "Follow-up",
            CreateCheckboxGroup::new(
                "followup",
                vec![CreateCheckboxGroupOption::new("Notify me", "notify")],
            )
            .min_values(2)
            .max_values(5),
        )),
        CreateModalComponent::Label(CreateLabel::checkbox(
            "Subscribe",
            CreateCheckbox::new("subscribe").default_selected(true),
        )),
    ]);

    assert_roundtrip::<CreateModalDe, _>(&modal);
}

#[test]
fn empty_modal_roundtrips() {
    let modal = CreateModal::new("ban:reason", "Ban reason");

    assert_roundtrip::<CreateModalDe, _>(&modal);
}

#[test]
fn label_with_description_roundtrips() {
    let modal = CreateModal::new("ticket:open", "Open a ticket").components(vec![
        CreateModalComponent::Label(
            CreateLabel::checkbox("escalate", CreateCheckbox::new("escalate"))
                .description("Pages the on-call engineer immediately"),
        ),
    ]);

    assert_roundtrip::<CreateModalDe, _>(&modal);
}

#[test]
fn input_text_with_every_field_set_roundtrips() {
    let modal = CreateModal::new("report:submit", "Report message").components(vec![
        CreateModalComponent::Label(CreateLabel::input_text(
            "Details",
            CreateInputText::new(InputTextStyle::Paragraph, "details")
                .min_length(10)
                .max_length(1000)
                .required(false)
                .value("Pre-filled from selection")
                .placeholder("What happened?"),
        )),
    ]);

    assert_roundtrip::<CreateModalDe, _>(&modal);
}

#[test]
fn file_upload_with_file_types_roundtrips() {
    let modal = CreateModal::new("appeal:upload", "Upload evidence").components(vec![
        CreateModalComponent::Label(CreateLabel::file_upload(
            "Evidence",
            CreateFileUpload::new("evidence").file_types(vec![
                Cow::<str>::Owned("image".into()),
                Cow::<str>::Owned(".pdf".into()),
            ]),
        )),
    ]);

    assert_roundtrip::<CreateModalDe, _>(&modal);
}

#[test]
fn full_modal_payload_roundtrips() {
    // Executable spec of the wire shape serenity emits for a filled-out
    // feedback modal.
    let payload = serde_json::json!({
        "components": [
            { "type": 10, "content": "**Mod application**" },
            {
                "type": 18,
                "label": "Experience",
                "description": "Prior moderation work",
                "component": {
                    "type": 4,
                    "custom_id": "experience",
                    "style": 2,
                    "min_length": 20,
                    "max_length": 800,
                    "required": true,
                    "placeholder": "Where have you moderated?",
                },
            },
            {
                "type": 18,
                "label": "Timezone",
                "description": null,
                "component": {
                    "type": 21,
                    "custom_id": "timezone",
                    "options": [
                        { "label": "UTC−08:00", "value": "utc-8" },
                        { "label": "UTC", "value": "utc", "description": "Coordinated Universal Time", "default": true },
                    ],
                    "required": false,
                },
            },
        ],
        "custom_id": "mod:apply",
        "title": "Moderator application",
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateModalDe, CreateModal<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- Documented canonicalizations ----

#[test]
fn unset_input_text_lengths_rebuild_as_explicit_nulls() {
    // Upstream has no skip attribute on min_length/max_length; the label's
    // description likewise always serializes.
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Name",
            "component": { "type": 4, "custom_id": "name", "style": 1, "required": true },
        }],
        "custom_id": "form",
        "title": "Form",
    });
    let expected = json!({
        "components": [{
            "type": 18,
            "label": "Name",
            "description": null,
            "component": {
                "type": 4,
                "custom_id": "name",
                "style": 1,
                "min_length": null,
                "max_length": null,
                "required": true,
            },
        }],
        "custom_id": "form",
        "title": "Form",
    });

    let rebuilt = rebuild::<CreateModalDe, CreateModal<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn unset_label_description_rebuilds_as_explicit_null() {
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Nickname",
            "component": { "type": 23, "custom_id": "nick" },
        }],
        "custom_id": "profile",
        "title": "Profile",
    });
    let expected = json!({
        "components": [{
            "type": 18,
            "label": "Nickname",
            "description": null,
            "component": { "type": 23, "custom_id": "nick" },
        }],
        "custom_id": "profile",
        "title": "Profile",
    });

    let rebuilt = rebuild::<CreateModalDe, CreateModal<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn empty_file_types_normalize_to_absent() {
    // Upstream skips empty file_types arrays entirely; the label's
    // description always serializes.
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Attachment",
            "component": {
                "type": 19,
                "custom_id": "attachment",
                "min_values": 1,
                "max_values": 1,
                "required": true,
                "file_types": [],
            },
        }],
        "custom_id": "upload",
        "title": "Upload",
    });
    let expected = json!({
        "components": [{
            "type": 18,
            "label": "Attachment",
            "description": null,
            "component": {
                "type": 19,
                "custom_id": "attachment",
                "min_values": 1,
                "max_values": 1,
                "required": true,
            },
        }],
        "custom_id": "upload",
        "title": "Upload",
    });

    let rebuilt = rebuild::<CreateModalDe, CreateModal<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn checkbox_group_zero_min_values_gains_required_false() {
    // Upstream couples its setters: min_values(0) force-enables
    // required = false, which then serializes explicitly.
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Extras",
            "component": {
                "type": 22,
                "custom_id": "extras",
                "options": [{ "label": "None", "value": "none" }],
                "min_values": 0,
            },
        }],
        "custom_id": "extras_form",
        "title": "Extras",
    });
    let expected = json!({
        "components": [{
            "type": 18,
            "label": "Extras",
            "description": null,
            "component": {
                "type": 22,
                "custom_id": "extras",
                "options": [{ "label": "None", "value": "none" }],
                "min_values": 0,
                "required": false,
            },
        }],
        "custom_id": "extras_form",
        "title": "Extras",
    });

    let rebuilt = rebuild::<CreateModalDe, CreateModal<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn checkbox_group_required_true_bumps_zero_min_values_to_one() {
    // Upstream's required(true) setter rewrites a zero min_values to one.
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Pick one",
            "component": {
                "type": 22,
                "custom_id": "pick",
                "options": [{ "label": "A", "value": "a" }],
                "min_values": 0,
                "required": true,
            },
        }],
        "custom_id": "pick_form",
        "title": "Pick",
    });
    let expected = json!({
        "components": [{
            "type": 18,
            "label": "Pick one",
            "description": null,
            "component": {
                "type": 22,
                "custom_id": "pick",
                "options": [{ "label": "A", "value": "a" }],
                "min_values": 1,
                "required": true,
            },
        }],
        "custom_id": "pick_form",
        "title": "Pick",
    });

    let rebuilt = rebuild::<CreateModalDe, CreateModal<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- Error paths ----

#[test]
fn modal_with_message_only_component_errors() {
    // Containers (17) exist only in messages, not modals.
    let payload = json!({
        "components": [{ "type": 17, "components": [] }],
        "custom_id": "bad",
        "title": "Bad",
    });

    let result: Result<CreateModalDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "a container is not a modal component");
}

#[test]
fn label_with_button_component_errors() {
    // Buttons (2) cannot live inside labels.
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Clicker",
            "component": { "type": 2, "style": 1, "custom_id": "nope" },
        }],
        "custom_id": "bad_label",
        "title": "Bad label",
    });

    let result: Result<CreateModalDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "a button is not a label component");
}

#[test]
fn label_without_component_errors() {
    let payload = json!({
        "components": [{ "type": 18, "label": "Empty" }],
        "custom_id": "empty_label",
        "title": "Empty",
    });

    let result: Result<CreateModalDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "a label without a component must be rejected"
    );
}

#[test]
fn input_text_without_required_flag_errors() {
    // Upstream always serializes `required`.
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Name",
            "component": { "type": 4, "custom_id": "name", "style": 1 },
        }],
        "custom_id": "form",
        "title": "Form",
    });

    let result: Result<CreateModalDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "\"required\" is always serialized upstream"
    );
}

#[test]
fn file_upload_without_always_present_fields_errors() {
    let payload = json!({
        "components": [{
            "type": 18,
            "label": "Attachment",
            "component": { "type": 19, "custom_id": "attachment", "min_values": 1 },
        }],
        "custom_id": "upload",
        "title": "Upload",
    });

    let result: Result<CreateModalDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "missing max_values/required must be rejected"
    );
}

// ---- Boundary values ----

#[test]
fn modal_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        // Empty strings everywhere they are allowed.
        json!({
            "components": [],
            "custom_id": "",
            "title": "",
        }),
        // Input text at the u16 length limits plus an unknown style number
        // (InputTextStyle falls back to Unknown(u8) upstream, spec D5).
        json!({
            "components": [{
                "type": 18,
                "label": "Limits",
                "description": null,
                "component": {
                    "type": 4,
                    "custom_id": "limits",
                    "style": 9,
                    "min_length": 0,
                    "max_length": 65535,
                    "required": false,
                    "value": "",
                },
            }],
            "custom_id": "limits_form",
            "title": "Limits",
        }),
        // File upload at the u8 boundaries.
        json!({
            "components": [{
                "type": 18,
                "label": "Many files",
                "description": null,
                "component": {
                    "type": 19,
                    "custom_id": "many",
                    "min_values": 0,
                    "max_values": 255,
                    "required": true,
                },
            }],
            "custom_id": "many_files",
            "title": "Files",
        }),
        // Checkbox default false must stay explicit, not vanish like an
        // omitted default.
        json!({
            "components": [{
                "type": 18,
                "label": "Opt-in",
                "description": null,
                "component": { "type": 23, "custom_id": "optin", "default": false },
            }],
            "custom_id": "optin_form",
            "title": "Opt-in",
        }),
        // Checkbox group at the u8 value bounds with every optional key set.
        json!({
            "components": [{
                "type": 18,
                "label": "Bounds",
                "description": null,
                "component": {
                    "type": 22,
                    "custom_id": "bounds",
                    "options": [
                        { "label": "", "value": "", "description": "", "default": false },
                    ],
                    "min_values": 255,
                    "max_values": 255,
                    "required": true,
                },
            }],
            "custom_id": "bounds_form",
            "title": "Bounds",
        }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateModalDe, CreateModal<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}
