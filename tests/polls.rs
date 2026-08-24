//! Round-trip suite for the poll family (`CreatePoll` collapsed to the
//! `Ready` typestate stage, answers and their private media mirrors) and
//! the soundboard builder.

use std::time::Duration;

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreatePoll;
use serenity::builder::CreatePollAnswer;
use serenity::model::channel::PollLayoutType;
use serenity::model::id::EmojiId;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

// ---- CreatePoll ----

#[test]
fn poll_with_every_field_set_roundtrips() {
    let poll = CreatePoll::new()
        .question("Cats or dogs?")
        .answers(vec![
            CreatePollAnswer::new().text("Cats!").emoji("🐱"),
            CreatePollAnswer::new().emoji(EmojiId::new(1_092_483_192_738_291_020)),
            CreatePollAnswer::new().text("Neither..."),
        ])
        .duration(Duration::from_secs(60 * 60 * 24 * 7))
        .allow_multiselect()
        .layout_type(PollLayoutType::Default);

    assert_roundtrip::<CreatePollDe, _>(&poll);
}

#[test]
fn minimal_poll_roundtrips() {
    let poll = CreatePoll::new()
        .question("Deploy on Friday?")
        .answers(vec![CreatePollAnswer::new().text("No")])
        .duration(Duration::from_secs(60 * 60));

    assert_roundtrip::<CreatePollDe, _>(&poll);
}

#[test]
fn unset_layout_type_rebuilds_as_explicit_null() {
    // Upstream has no skip attribute on layout_type; poll media fields
    // always serialize too.
    let payload = json!({
        "question": { "text": "Tea or coffee?" },
        "answers": [{ "poll_media": { "text": "Coffee", "emoji": null } }],
        "duration": 24,
        "allow_multiselect": false,
        "layout_type": null,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreatePollDe, CreatePoll<'static, serenity::builder::create_poll::Ready>>(
        &payload,
    );

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn answer_media_unset_fields_rebuild_as_explicit_nulls() {
    // Upstream serializes text/emoji as explicit nulls when unset.
    let payload = json!({
        "question": { "text": "Ping?" },
        "answers": [{ "poll_media": {} }],
        "duration": 1,
        "allow_multiselect": false,
        "layout_type": null,
    });
    let expected = json!({
        "question": { "text": "Ping?" },
        "answers": [{ "poll_media": { "text": null, "emoji": null } }],
        "duration": 1,
        "allow_multiselect": false,
        "layout_type": null,
    });

    let rebuilt = rebuild::<CreatePollDe, CreatePoll<'static, serenity::builder::create_poll::Ready>>(
        &payload,
    );

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn custom_emoji_answer_roundtrips_from_raw_payload() {
    // Discord wire shape: emoji ids as strings.
    let payload = json!({
        "question": { "text": "Best release?" },
        "answers": [
            { "poll_media": { "text": "2026.31", "emoji": { "id": "1092483192738291020" } } },
            { "poll_media": { "text": "2026.34", "emoji": { "name": "🚀" } } },
        ],
        "duration": 48,
        "allow_multiselect": true,
        "layout_type": 1,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreatePollDe, CreatePoll<'static, serenity::builder::create_poll::Ready>>(
        &payload,
    );

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn unknown_layout_type_survives_the_roundtrip() {
    // PollLayoutType falls back to Unknown(u8) upstream (spec D5).
    let payload = json!({
        "question": { "text": "Which layout?" },
        "answers": [{ "poll_media": { "text": "Mystery", "emoji": null } }],
        "duration": 6,
        "allow_multiselect": false,
        "layout_type": 9,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreatePollDe, CreatePoll<'static, serenity::builder::create_poll::Ready>>(
        &payload,
    );

    assert_value_eq(&expected, &rebuilt);
}

// ---- Error paths ----

#[test]
fn poll_without_duration_errors() {
    let payload = json!({
        "question": { "text": "No duration?" },
        "answers": [{ "poll_media": { "text": "Aye" } }],
        "allow_multiselect": false,
        "layout_type": null,
    });

    let result: Result<CreatePollDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "\"duration\" is always serialized upstream"
    );
}

#[test]
fn poll_without_allow_multiselect_errors() {
    let payload = json!({
        "question": { "text": "Multi?" },
        "answers": [{ "poll_media": { "text": "One" } }],
        "duration": 12,
        "layout_type": null,
    });

    let result: Result<CreatePollDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "\"allow_multiselect\" is always serialized upstream"
    );
}

#[test]
fn negative_duration_errors() {
    let payload = json!({
        "question": { "text": "Time travel?" },
        "answers": [{ "poll_media": { "text": "Backwards" } }],
        "duration": -1,
        "allow_multiselect": false,
        "layout_type": null,
    });

    let result: Result<CreatePollDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "a negative hour count must be rejected");
}

#[test]
fn answer_without_poll_media_errors() {
    let payload = json!({
        "question": { "text": "Empty choice?" },
        "answers": [{}],
        "duration": 12,
        "allow_multiselect": false,
        "layout_type": null,
    });

    let result: Result<CreatePollDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "an answer without \"poll_media\" must be rejected"
    );
}

// ---- Boundary values ----

#[test]
fn poll_duration_boundaries_roundtrip() {
    // 0 hours, the upstream 32-day cap (768), and u16::MAX.
    let cases: Vec<Value> = [0u16, 768, 65535]
        .into_iter()
        .map(|hours| {
            json!({
                "question": { "text": "" },
                "answers": [{ "poll_media": { "text": "", "emoji": null } }],
                "duration": hours,
                "allow_multiselect": true,
                "layout_type": null,
            })
        })
        .collect();

    for payload in cases {
        let rebuilt = rebuild::<
            CreatePollDe,
            CreatePoll<'static, serenity::builder::create_poll::Ready>,
        >(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

// ---- CreateSoundboard ----

#[test]
fn soundboard_with_every_field_set_roundtrips() {
    let soundboard = serenity::builder::CreateSoundboard::new(
        "airhorn",
        serenity::builder::DataUri::from_base64("data:audio/ogg;base64,T2dnUwACAAAAAAAAAAA=")
            .expect("valid data URI"),
    )
    .volume(0.75)
    .emoji_id(EmojiId::new(936_926_320_553_408_522))
    .emoji_name("loud");

    assert_roundtrip::<CreateSoundboardDe, _>(&soundboard);
}

#[test]
fn minimal_soundboard_roundtrips() {
    let soundboard = serenity::builder::CreateSoundboard::new(
        "beep",
        serenity::builder::DataUri::from_base64(
            "data:audio/mpeg;base64,SUQzBAAAAAAAI1RTU0UAAAAPAAADTGF2ZjU4Ljc2LjEwMAAAAAAAAAAAAAAA",
        )
        .expect("valid data URI"),
    );

    assert_roundtrip::<CreateSoundboardDe, _>(&soundboard);
}

#[test]
fn invalid_data_uri_errors_at_deserialization() {
    let cases: Vec<Value> = vec![
        json!("https://cdn.example.com/sound.ogg"),
        json!("data:no-slash-here;base64,QUJD"),
        json!("data:audio/ogg;hex,QUJD"),
        json!("plain noise without a scheme"),
        json!(""),
    ];

    for sound in cases {
        let payload = json!({
            "name": "broken",
            "sound": sound,
            "volume": 1.0,
        });

        let result: Result<CreateSoundboardDe, _> = serde_json::from_value(payload);

        assert!(
            result.is_err(),
            "invalid data URI must be rejected: {sound}"
        );
    }
}

#[test]
fn soundboard_boundary_volumes_and_absent_emojis_roundtrip() {
    let cases: Vec<Value> = vec![
        json!({
            "name": "silence",
            "sound": "data:audio/ogg;base64,QUJD",
            "volume": 0.0,
        }),
        json!({
            "name": "full blast",
            "sound": "data:audio/ogg;base64,QUJD",
            "volume": 1.0,
        }),
        json!({
            "name": "",
            "sound": "data:a/b;base64,x",
            "volume": 0.5,
            "emoji_id": "18446744073709551614",
            "emoji_name": "",
        }),
    ];

    for payload in cases {
        let expected = payload.clone();
        let rebuilt =
            rebuild::<CreateSoundboardDe, serenity::builder::CreateSoundboard<'static>>(&payload);
        assert_value_eq(&expected, &rebuilt);
    }
}
