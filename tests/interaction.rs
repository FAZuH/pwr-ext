//! Round-trip suite for the interaction-response family: the tagged
//! `CreateInteractionResponse` enum, its message payload, followups, and
//! autocomplete responses with their choice mirrors.

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateActionRow;
use serenity::builder::CreateAutocompleteResponse;
use serenity::builder::CreateButton;
use serenity::builder::CreateComponent;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateInteractionResponse;
use serenity::builder::CreateInteractionResponseFollowup;
use serenity::builder::CreateInteractionResponseMessage;
use serenity::builder::CreateLabel;
use serenity::builder::CreateModal;
use serenity::model::application::InputTextStyle;
use serenity::model::channel::MessageFlags;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

// ---- CreateInteractionResponse ----

#[test]
fn pong_acknowledge_and_launch_responses_roundtrip() {
    // Kind-only responses serialize an explicit null data field.
    assert_roundtrip::<CreateInteractionResponseDe, _>(&CreateInteractionResponse::Pong);
    assert_roundtrip::<CreateInteractionResponseDe, _>(&CreateInteractionResponse::Acknowledge);
    assert_roundtrip::<CreateInteractionResponseDe, _>(&CreateInteractionResponse::LaunchActivity);
}

#[test]
fn deferred_update_with_full_message_roundtrips() {
    let response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::new()
            .tts(false)
            .content("Ticket #4021 updated")
            .embed(CreateEmbed::new().title("Status").description("Resolved"))
            .flags(MessageFlags::SUPPRESS_EMBEDS)
            .components(vec![CreateComponent::ActionRow(CreateActionRow::buttons(
                vec![CreateButton::new("ticket:close").label("Close")],
            ))]),
    );

    assert_roundtrip::<CreateInteractionResponseDe, _>(&response);
}

#[test]
fn defer_response_with_empty_payload_roundtrips() {
    let response = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());

    assert_roundtrip::<CreateInteractionResponseDe, _>(&response);
}

#[test]
fn modal_response_reuses_the_modal_mirror() {
    let response = CreateInteractionResponse::Modal(
        CreateModal::new("feedback:form", "Send feedback").components(vec![
            serenity::builder::CreateModalComponent::TextDisplay(
                serenity::builder::CreateTextDisplay::new("**Feedback** — tell us more"),
            ),
            serenity::builder::CreateModalComponent::Label(CreateLabel::input_text(
                "Summary",
                serenity::builder::CreateInputText::new(InputTextStyle::Short, "summary"),
            )),
        ]),
    );

    assert_roundtrip::<CreateInteractionResponseDe, _>(&response);
}

#[test]
fn response_kind_tags_are_pinned_by_wire_shape() {
    let cases: Vec<Value> = vec![
        json!({ "type": 1, "data": null }),
        json!({ "type": 4, "data": { "attachments": [] } }),
        json!({ "type": 5, "data": { "attachments": [] } }),
        json!({ "type": 6, "data": null }),
        json!({ "type": 7, "data": { "content": "edited", "attachments": [] } }),
        json!({ "type": 12, "data": null }),
    ];

    for payload in cases {
        let rebuilt =
            rebuild::<CreateInteractionResponseDe, CreateInteractionResponse<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn unknown_response_kind_errors() {
    // The upstream enum is a fixed match with no Unknown fallback (spec D5).
    for kind in [2u64, 3, 10, 11, 99] {
        let payload = json!({ "type": kind, "data": null });

        let result: Result<CreateInteractionResponseDe, _> = serde_json::from_value(payload);

        assert!(result.is_err(), "kind {kind} is not representable upstream");
    }
}

#[test]
fn response_without_data_key_errors() {
    let payload = json!({ "type": 4 });

    let result: Result<CreateInteractionResponseDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "\"data\" is always serialized upstream");
}

#[test]
fn response_message_attachments_are_accepted_but_dropped_on_rebuild() {
    let payload = json!({
        "type": 4,
        "data": {
            "content": "Report attached",
            "attachments": [
                { "id": 0, "filename": "q2-report.pdf", "is_spoiler": false },
            ],
        },
    });
    let expected = json!({
        "type": 4,
        "data": {
            "content": "Report attached",
            "attachments": [],
        },
    });

    let rebuilt =
        rebuild::<CreateInteractionResponseDe, CreateInteractionResponse<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- CreateAutocompleteResponse ----

#[test]
fn autocomplete_response_with_every_value_kind_roundtrips() {
    let response = CreateAutocompleteResponse::new()
        .add_choice(serenity::builder::AutocompleteChoice::new("plain", "text"))
        .add_choice(serenity::builder::AutocompleteChoice::new(
            "counted", 25_u64,
        ))
        .add_choice(serenity::builder::AutocompleteChoice::new(
            "precise", 1.5_f64,
        ));

    assert_roundtrip::<CreateAutocompleteResponseDe, _>(&response);
}

#[test]
fn localized_autocomplete_choices_roundtrip() {
    let response = CreateAutocompleteResponse::new().add_choice(
        serenity::builder::AutocompleteChoice::new("region", "eu-central")
            .add_localized_name("de-DE", "Region")
            .add_localized_name("zh-CN", "区域"),
    );

    assert_roundtrip::<CreateAutocompleteResponseDe, _>(&response);
}

#[test]
fn negative_autocomplete_integers_normalize_to_floats() {
    // Upstream can only build Integer(u64) or Float(f64); a negative JSON
    // integer therefore rebuilds through the closest form, the float path.
    let payload = json!({
        "choices": [
            { "name": "minus", "value": -5 },
            { "name": "integral float", "value": 2.0 },
        ],
    });
    let expected = json!({
        "choices": [
            { "name": "minus", "value": -5.0 },
            { "name": "integral float", "value": 2.0 },
        ],
    });

    let rebuilt =
        rebuild::<CreateAutocompleteResponseDe, CreateAutocompleteResponse<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn autocomplete_values_of_wrong_json_kind_errors() {
    let values: Vec<Value> = vec![json!(true), json!([1]), json!({ "n": 1 }), Value::Null];

    for value in values {
        let payload = json!({ "choices": [{ "name": "broken", "value": value }] });

        let result: Result<CreateAutocompleteResponseDe, _> = serde_json::from_value(payload);

        assert!(
            result.is_err(),
            "a {value} autocomplete value is not representable upstream"
        );
    }
}

#[test]
fn autocomplete_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        // Empty choice list plus empty strings.
        json!({ "choices": [{ "name": "", "value": "" }] }),
        json!({ "choices": [] }),
        // Integer upper bound and a float that keeps its decimal point.
        json!({ "choices": [{ "name": "max", "value": 18446744073709551615u64 }] }),
        json!({ "choices": [{ "name": "zero float", "value": 0.0 }] }),
    ];

    for payload in cases {
        let rebuilt =
            rebuild::<CreateAutocompleteResponseDe, CreateAutocompleteResponse<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

// ---- CreateInteractionResponseFollowup ----

#[test]
fn followup_with_every_field_set_roundtrips() {
    let followup = CreateInteractionResponseFollowup::new()
        .content("Follow-up details")
        .tts(false)
        .embed(CreateEmbed::new().title("Follow-up"))
        .allowed_mentions(pwr_allowed_mentions())
        .flags(MessageFlags::EPHEMERAL)
        .components(vec![CreateComponent::ActionRow(CreateActionRow::buttons(
            vec![CreateButton::new_link("https://status.example.com")],
        ))]);

    assert_roundtrip::<CreateInteractionResponseFollowupDe, _>(&followup);
}

fn pwr_allowed_mentions() -> serenity::builder::CreateAllowedMentions<'static> {
    serenity::builder::CreateAllowedMentions::new().everyone(true)
}

#[test]
fn minimal_followup_roundtrips_byte_stable() {
    let payload = json!({
        "content": "later",
        "embeds": null,
        "attachments": [],
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<
        CreateInteractionResponseFollowupDe,
        CreateInteractionResponseFollowup<'static>,
    >(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn followup_missing_embeds_rebuild_as_explicit_null() {
    // Unlike the response message, the followup serializer has no skip
    // attribute on embeds.
    let payload = json!({
        "content": "no embeds here",
    });
    let expected = json!({
        "content": "no embeds here",
        "embeds": null,
        "attachments": [],
    });

    let rebuilt = rebuild::<
        CreateInteractionResponseFollowupDe,
        CreateInteractionResponseFollowup<'static>,
    >(&payload);

    assert_value_eq(&expected, &rebuilt);
}
