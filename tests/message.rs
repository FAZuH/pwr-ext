//! Round-trip suite for the message payload family: `CreateMessage` and
//! `CreateAllowedMentions`.

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateActionRow;
use serenity::builder::CreateAllowedMentions;
use serenity::builder::CreateButton;
use serenity::builder::CreateComponent;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateMessage;
use serenity::model::application::ButtonStyle;
use serenity::model::channel::MessageFlags;
use serenity::model::channel::MessageReference;
use serenity::model::channel::MessageReferenceKind;
use serenity::model::channel::Nonce;
use serenity::model::id::GenericChannelId;
use serenity::model::id::GuildId;
use serenity::model::id::MessageId;
use serenity::model::id::RoleId;
use serenity::model::id::StickerId;
use serenity::model::id::UserId;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

// ---- CreateMessage ----

#[test]
fn message_with_every_field_set_roundtrips() {
    let embed = CreateEmbed::new()
        .title("Release 2026.34")
        .description("Rolled out to all regions without incident.")
        .field("Latency", "-12ms p95", true);
    let message = CreateMessage::new()
        .content("Release train deployed")
        .nonce(Nonce::Number(1_779_521_234_567_890))
        .embed(embed)
        .allowed_mentions(
            CreateAllowedMentions::new()
                .everyone(true)
                .push_user(UserId::new(110_372_470_472_613_888))
                .push_role(RoleId::new(182_894_738_100_322_304))
                .replied_user(true),
        )
        .reference_message(
            MessageReference::new(
                MessageReferenceKind::Default,
                GenericChannelId::new(482_842_547_397_828_608),
            )
            .message_id(MessageId::new(1_293_847_564_839_201_984))
            .guild_id(GuildId::new(902_361_593_214_052_352))
            .fail_if_not_exists(false),
        )
        .components(vec![CreateComponent::ActionRow(CreateActionRow::buttons(
            vec![
                CreateButton::new("release:acknowledge")
                    .label("Acknowledge")
                    .style(ButtonStyle::Success),
            ],
        ))])
        .sticker_ids(vec![StickerId::new(1_092_483_192_738_291_020)])
        .flags(MessageFlags::SUPPRESS_EMBEDS)
        .enforce_nonce(true);

    assert_roundtrip::<CreateMessageDe, _>(&message);
}

#[test]
fn empty_message_roundtrips() {
    let message = CreateMessage::new();

    assert_roundtrip::<CreateMessageDe, _>(&message);
}

#[test]
fn content_only_message_roundtrips() {
    let message = CreateMessage::new().content("Deploy finished — see thread for details");

    assert_roundtrip::<CreateMessageDe, _>(&message);
}

#[test]
fn forwarded_message_reference_roundtrips() {
    let message = CreateMessage::new().reference_message(
        MessageReference::new(MessageReferenceKind::Forward, GenericChannelId::new(9))
            .message_id(MessageId::new(42)),
    );

    assert_roundtrip::<CreateMessageDe, _>(&message);
}

#[test]
fn v2_flagged_components_only_payload_roundtrips() {
    // IS_COMPONENTS_V2 == 1 << 15 == 32768; v2 messages cannot carry
    // `content` or `embeds`, so the wire payload is components-only.
    let payload = json!({
        "tts": false,
        "embeds": [],
        "components": [{
            "type": 17,
            "accent_color": 5793266,
            "components": [
                { "type": 10, "content": "**Incident #4021** review" },
                { "type": 14 },
            ],
        }],
        "sticker_ids": [],
        "flags": 32768,
        "attachments": [],
        "enforce_nonce": false,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateMessageDe, CreateMessage<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- Documented canonicalizations ----

#[test]
fn unknown_flag_bits_are_truncated_like_upstream() {
    // Upstream deserializes flags via from_bits_truncate: bits 9-11 are not
    // defined on MessageFlags and normalize away instead of erroring.
    let payload = json!({
        "tts": false,
        "embeds": [],
        "sticker_ids": [],
        "attachments": [],
        "enforce_nonce": false,
        "flags": 65535u16,
    });
    let expected = json!({
        "tts": false,
        "embeds": [],
        "sticker_ids": [],
        "attachments": [],
        "enforce_nonce": false,
        "flags": 61951u16,
    });

    let rebuilt = rebuild::<CreateMessageDe, CreateMessage<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn attachments_array_is_accepted_but_dropped_on_rebuild() {
    // The multipart attachment plumbing is excluded from round-trip support
    // (spec D4): rebuilt messages always emit an empty array.
    let payload = json!({
        "content": "Quarterly report attached",
        "tts": false,
        "embeds": [],
        "sticker_ids": [],
        "attachments": [
            {
                "id": 0,
                "filename": "q2-report.pdf",
                "description": "Q2 financial summary",
                "is_spoiler": false,
            },
        ],
        "enforce_nonce": false,
    });
    let expected = json!({
        "content": "Quarterly report attached",
        "tts": false,
        "embeds": [],
        "sticker_ids": [],
        "attachments": [],
        "enforce_nonce": false,
    });

    let rebuilt = rebuild::<CreateMessageDe, CreateMessage<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn full_message_with_poll_roundtrips() {
    // Executable spec of the wire shape: the nested poll carries upstream's
    // explicit nulls (answer media text/emoji, layout_type).
    let payload = json!({
        "content": "Weekly community poll",
        "tts": false,
        "embeds": [],
        "poll": {
            "question": { "text": "Pineapple on pizza?" },
            "answers": [
                { "poll_media": { "text": "Yes", "emoji": { "name": "🍕" } } },
                { "poll_media": { "text": "No", "emoji": null } },
                { "poll_media": { "text": null, "emoji": { "id": "1092483192738291020" } } },
            ],
            "duration": 24,
            "allow_multiselect": false,
            "layout_type": null,
        },
        "sticker_ids": [],
        "attachments": [],
        "enforce_nonce": false,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateMessageDe, CreateMessage<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn omitted_embeds_and_sticker_ids_rebuild_as_explicit_empty_arrays() {
    // Upstream always serializes both arrays, so input omitting them
    // canonically gains `"embeds": []` / `"sticker_ids": []`.
    let payload = json!({
        "content": "just text",
        "tts": false,
        "attachments": [],
        "enforce_nonce": false,
    });
    let expected = json!({
        "content": "just text",
        "tts": false,
        "embeds": [],
        "sticker_ids": [],
        "attachments": [],
        "enforce_nonce": false,
    });

    let rebuilt = rebuild::<CreateMessageDe, CreateMessage<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- Error paths ----

#[test]
fn message_without_tts_errors() {
    let payload = json!({
        "content": "no tts field",
        "embeds": [],
        "sticker_ids": [],
        "attachments": [],
        "enforce_nonce": false,
    });

    let result: Result<CreateMessageDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "\"tts\" is always serialized upstream");
}

#[test]
fn message_without_enforce_nonce_errors() {
    let payload = json!({
        "content": "no enforce_nonce field",
        "tts": false,
        "embeds": [],
        "sticker_ids": [],
        "attachments": [],
    });

    let result: Result<CreateMessageDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "\"enforce_nonce\" is always serialized upstream"
    );
}

#[test]
fn non_array_attachments_errors() {
    let payload = json!({
        "tts": false,
        "embeds": [],
        "sticker_ids": [],
        "attachments": { "id": 0 },
        "enforce_nonce": false,
    });

    let result: Result<CreateMessageDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "\"attachments\" must be a JSON array");
}

// ---- Boundary values ----

#[test]
fn message_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        json!({
            "content": "",
            "tts": false,
            "embeds": [],
            "sticker_ids": [],
            "attachments": [],
            "enforce_nonce": false,
        }),
        json!({
            "content": "abcdefghij1234567890ABCDE",
            "nonce": "abcdefghij1234567890ABCDE",
            "tts": true,
            "embeds": [],
            "sticker_ids": [],
            "attachments": [],
            "enforce_nonce": true,
        }),
        json!({
            "nonce": 18446744073709551615u64,
            "tts": true,
            "embeds": [],
            "sticker_ids": [],
            "attachments": [],
            "enforce_nonce": true,
        }),
        json!({
            "tts": false,
            "embeds": [],
            "components": [],
            "sticker_ids": [],
            "attachments": [],
            "enforce_nonce": false,
        }),
        json!({
            "tts": false,
            "embeds": [],
            // IDs serialize back as strings (upstream accepts either form
            // on deserialize); u64::MAX - 1 is the largest snowflake
            // upstream accepts.
            "sticker_ids": ["18446744073709551614"],
            "attachments": [],
            "enforce_nonce": false,
            "flags": 0u8,
        }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateMessageDe, CreateMessage<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

// ---- CreateAllowedMentions ----

#[test]
fn allowed_mentions_with_every_field_set_roundtrips() {
    let mentions = CreateAllowedMentions::new()
        .everyone(true)
        .all_users(true)
        .all_roles(true)
        .users(vec![
            UserId::new(110_372_470_472_613_888),
            UserId::new(182_891_574_139_682_816),
        ])
        .roles(vec![RoleId::new(182_894_738_100_322_304)])
        .replied_user(true);

    assert_roundtrip::<CreateAllowedMentionsDe, _>(&mentions);
}

#[test]
fn default_allowed_mentions_roundtrips() {
    let mentions = CreateAllowedMentions::new();

    assert_roundtrip::<CreateAllowedMentionsDe, _>(&mentions);
}

#[test]
fn parse_strings_rebuild_in_original_order() {
    // Rebuilding replays the parse entries in payload order; applying the
    // toggles in a fixed order would reorder them.
    let payload = json!({
        "parse": ["roles", "everyone", "users"],
        "users": [],
        "roles": [],
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateAllowedMentionsDe, CreateAllowedMentions>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn duplicate_parse_entries_collapse_to_one() {
    // Upstream's parse list holds each value at most once (the toggle
    // setters dedupe), so replaying duplicates collapses rather than errors.
    let payload = json!({
        "parse": ["users", "users"],
        "users": [],
        "roles": [],
    });
    let expected = json!({
        "parse": ["users"],
        "users": [],
        "roles": [],
    });

    let rebuilt = rebuild::<CreateAllowedMentionsDe, CreateAllowedMentions>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn replied_user_false_survives_the_roundtrip() {
    // Some(false) must stay an explicit false, not be treated as absent.
    let payload = json!({
        "parse": [],
        "users": [],
        "roles": [],
        "replied_user": false,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateAllowedMentionsDe, CreateAllowedMentions>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn omitted_allowed_mentions_arrays_rebuild_as_explicit_empty_arrays() {
    let payload = json!({});
    let expected = json!({
        "parse": [],
        "users": [],
        "roles": [],
    });

    let rebuilt = rebuild::<CreateAllowedMentionsDe, CreateAllowedMentions>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn unknown_parse_string_errors() {
    // Upstream's private ParseValue enum has no unknown-value fallback (D5).
    let payload = json!({
        "parse": ["channels"],
        "users": [],
        "roles": [],
    });

    let result: Result<CreateAllowedMentionsDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "\"channels\" is not a valid parse value");
}

#[test]
fn allowed_mentions_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        json!({ "parse": ["everyone"], "users": [], "roles": [] }),
        json!({
            "parse": ["users", "roles"],
            "users": ["110372470472613888"],
            "roles": ["182894738100322304"],
        }),
        json!({
            "parse": [],
            "users": ["18446744073709551614"],
            "roles": ["1"],
            "replied_user": true,
        }),
        json!({ "parse": [], "users": [], "roles": [], "replied_user": false }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateAllowedMentionsDe, CreateAllowedMentions>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}
