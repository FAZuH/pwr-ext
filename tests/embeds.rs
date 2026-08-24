//! Round-trip suite for the embed family: `CreateEmbed` and its author and
//! footer builders, plus the private field/image mirrors.

use std::borrow::Cow;

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateEmbedAuthor;
use serenity::builder::CreateEmbedFooter;
use serenity::model::Colour;
use serenity::model::Timestamp;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

// ---- CreateEmbed ----

#[test]
fn embed_with_every_field_set_roundtrips() {
    let embed = CreateEmbed::new()
        .title("Incident #4021")
        .url("https://status.example.com/incidents/4021")
        .description("API latency elevated in eu-central.")
        .colour(Colour::new(0xDE_2C_43))
        .timestamp(
            "2026-08-23T16:04:23Z"
                .parse::<Timestamp>()
                .expect("valid RFC 3339"),
        )
        .author(
            CreateEmbedAuthor::new("Status Bot")
                .url("https://status.example.com")
                .icon_url("https://cdn.example.com/bot.png"),
        )
        .footer(
            CreateEmbedFooter::new("Ops — status.example.com")
                .icon_url("https://cdn.example.com/footer.png"),
        )
        .image(
            "https://cdn.example.com/graph.png",
            Some(Cow::Owned("5xx rate by region".to_string())),
        )
        .thumbnail("https://cdn.example.com/logo.png", None)
        .field("Region", "eu-central-1", true)
        .field("Uptime", "99.92%", false);

    assert_roundtrip::<CreateEmbedDe<'static>, _>(&embed);
}

#[test]
fn title_only_embed_roundtrips() {
    let embed = CreateEmbed::new().title("Bare title");

    assert_roundtrip::<CreateEmbedDe<'static>, _>(&embed);
}

#[test]
fn empty_embed_roundtrips_as_kind_rich_only() {
    let embed = CreateEmbed::new();

    assert_roundtrip::<CreateEmbedDe<'static>, _>(&embed);
}

#[test]
fn embed_fields_roundtrip_in_order_and_keep_inline_flags() {
    let embed = CreateEmbed::new()
        .field("First", "1", false)
        .field("Second", "2", true)
        .field("Third", "3", true);

    let original = serde_json::to_value(&embed).expect("serializes");
    let rebuilt = rebuild::<CreateEmbedDe<'static>, CreateEmbed<'static>>(&original);

    assert_value_eq(&original, &rebuilt);
}

// ---- Documented canonicalizations (spec D7) ----

#[test]
fn embed_kind_is_always_rebuilt_as_rich() {
    // Upstream has no setter for `type`; every rebuilt embed is "rich",
    // whatever the input claimed.
    let payload = json!({
        "type": "image",
        "title": "Not really rich",
    });
    let expected = json!({
        "type": "rich",
        "title": "Not really rich",
    });

    let rebuilt = rebuild::<CreateEmbedDe<'static>, CreateEmbed<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn unset_author_urls_rebuild_as_explicit_nulls() {
    // Author url/icon_url serialize as explicit null when unset.
    let payload = json!({
        "type": "rich",
        "author": { "name": "No Links Bot" },
    });
    let expected = json!({
        "type": "rich",
        "author": {
            "name": "No Links Bot",
            "url": null,
            "icon_url": null,
        },
    });

    let rebuilt = rebuild::<CreateEmbedDe<'static>, CreateEmbed<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn unset_footer_icon_url_rebuilds_as_explicit_null() {
    let payload = json!({
        "type": "rich",
        "footer": { "text": "plain footer" },
    });
    let expected = json!({
        "type": "rich",
        "footer": { "text": "plain footer", "icon_url": null },
    });

    let rebuilt = rebuild::<CreateEmbedDe<'static>, CreateEmbed<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- Error paths ----

#[test]
fn embed_colour_outside_u32_range_errors() {
    let payload = json!({ "color": -1 });

    let result: Result<CreateEmbedDe<'static>, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "negative colour must be rejected");
}

#[test]
fn embed_timestamp_not_rfc3339_errors() {
    let payload = json!({ "timestamp": "yesterday-ish" });

    let result: Result<CreateEmbedDe<'static>, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "non-RFC 3339 timestamp must be rejected");
}

#[test]
fn embed_field_missing_inline_flag_errors() {
    // Upstream always serializes `inline`; the mirror requires it.
    let payload = json!({
        "fields": [{ "name": "A", "value": "B" }],
    });

    let result: Result<CreateEmbedDe<'static>, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "a field without \"inline\" must be rejected"
    );
}

#[test]
fn embed_author_missing_name_errors() {
    let payload = json!({
        "author": { "url": null, "icon_url": null },
    });

    let result: Result<CreateEmbedDe<'static>, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "an author without a name must be rejected");
}

#[test]
fn embed_image_missing_url_errors() {
    let payload = json!({ "image": { "description": "orphan alt text" } });

    let result: Result<CreateEmbedDe<'static>, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "an image without a url must be rejected");
}

// ---- Boundary values ----

#[test]
fn boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        json!({ "type": "rich", "color": 0 }),
        json!({ "type": "rich", "color": 4294967295u32 }),
        json!({ "type": "rich", "timestamp": "1970-01-01T00:00:00Z" }),
        json!({ "type": "rich", "title": "", "description": "", "url": "" }),
        json!({ "type": "rich", "fields": [{ "name": "", "value": "", "inline": false }] }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateEmbedDe<'static>, CreateEmbed<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}
