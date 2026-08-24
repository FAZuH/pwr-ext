//! Round-trip suite for the remaining JSON-emitting builders: welcome-screen
//! channels, role colours, and test entitlements.

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateGuildWelcomeChannel;
use serenity::builder::CreateRoleColours;
use serenity::builder::CreateTestEntitlement;
use serenity::model::Colour;
use serenity::model::guild::GuildWelcomeChannelEmoji;
use serenity::model::id::EmojiId;
use serenity::model::id::GenericChannelId;
use serenity::model::id::GuildId;
use serenity::model::id::SkuId;
use serenity::model::id::UserId;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

// ---- CreateGuildWelcomeChannel ----

#[test]
fn welcome_channel_with_custom_emoji_roundtrips() {
    let channel = CreateGuildWelcomeChannel::new(GenericChannelId::new(7), "Say hello!").emoji(
        GuildWelcomeChannelEmoji::Custom {
            id: EmojiId::new(936_926_320_553_408_522),
            name: "wave".to_owned().try_into().expect("short name"),
        },
    );

    assert_roundtrip::<CreateGuildWelcomeChannelDe, _>(&channel);
}

#[test]
fn welcome_channel_with_unicode_emoji_roundtrips() {
    let channel = CreateGuildWelcomeChannel::new(
        GenericChannelId::new(482_842_547_397_828_608),
        "Rules first, please",
    )
    .emoji(GuildWelcomeChannelEmoji::Unicode(
        "📜".to_owned().try_into().expect("short name"),
    ));

    assert_roundtrip::<CreateGuildWelcomeChannelDe, _>(&channel);
}

#[test]
fn welcome_channel_without_emoji_roundtrips() {
    let channel = CreateGuildWelcomeChannel::new(GenericChannelId::new(9), "General chat");

    assert_roundtrip::<CreateGuildWelcomeChannelDe, _>(&channel);
}

#[test]
fn welcome_channel_unset_emojis_rebuild_as_explicit_nulls() {
    // All four fields serialize unconditionally upstream.
    let payload = json!({
        "channel_id": "9",
        "description": "General chat",
    });
    let expected = json!({
        "channel_id": "9",
        "emoji_name": null,
        "emoji_id": null,
        "description": "General chat",
    });

    let rebuilt =
        rebuild::<CreateGuildWelcomeChannelDe, CreateGuildWelcomeChannel<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn welcome_channel_emoji_id_without_name_errors() {
    // The builder emoji setter takes a two-variant enum whose custom
    // variant always carries both id and name.
    let payload = json!({
        "channel_id": "7",
        "emoji_name": null,
        "emoji_id": "42",
        "description": "Broken",
    });

    let result: Result<CreateGuildWelcomeChannelDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "an emoji id without an emoji name is not representable"
    );
}

// ---- CreateRoleColours ----

#[test]
fn role_colours_with_every_field_set_roundtrips() {
    let colours = CreateRoleColours::new(Colour::new(0xDE_2C_43))
        .secondary_colour(Colour::new(0x58_65_F2))
        .tertiary_colour(Colour::new(0xEB_45_9E));

    assert_roundtrip::<CreateRoleColoursDe, _>(&colours);
}

#[test]
fn primary_only_role_colours_roundtrips() {
    let colours = CreateRoleColours::new(Colour::new(0x00_b0_f8));

    assert_roundtrip::<CreateRoleColoursDe, _>(&colours);
}

#[test]
fn role_colours_optional_fields_stay_absent_when_unset() {
    let payload = json!({ "primary_color": 14560323 });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateRoleColoursDe, CreateRoleColours>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- CreateTestEntitlement ----

#[test]
fn guild_owned_test_entitlement_roundtrips() {
    let entitlement = CreateTestEntitlement::new(
        SkuId::new(1_092_483_192_738_291_020),
        serenity::builder::EntitlementOwner::Guild(GuildId::new(902_361_593_214_052_352)),
    );

    assert_roundtrip::<CreateTestEntitlementDe, _>(&entitlement);
}

#[test]
fn user_owned_test_entitlement_roundtrips() {
    let entitlement = CreateTestEntitlement::new(
        SkuId::new(1_234_567_890_123_456_789),
        serenity::builder::EntitlementOwner::User(UserId::new(110_372_470_472_613_888)),
    );

    assert_roundtrip::<CreateTestEntitlementDe, _>(&entitlement);
}

#[test]
fn test_entitlement_unknown_owner_type_errors() {
    // Upstream builds owner_type from a two-variant owner enum; no other
    // number is constructible (spec D5).
    let payload = json!({
        "sku_id": "1",
        "owner_id": "2",
        "owner_type": 7,
    });

    let result: Result<CreateTestEntitlementDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "owner type 7 has no upstream constructor");
}

// ---- Boundary values ----

#[test]
fn welcome_channel_boundary_values_roundtrip() {
    // Colour-free channel with an empty emoji name and empty description,
    // plus the largest snowflake ids upstream accepts.
    let cases: Vec<Value> = vec![
        json!({
            "channel_id": "18446744073709551614",
            "emoji_name": "",
            "emoji_id": null,
            "description": "",
        }),
        json!({
            "channel_id": "1",
            "emoji_name": "🔥",
            "emoji_id": "18446744073709551614",
            "description": "both emoji fields set is representable here",
        }),
    ];

    for payload in cases {
        let rebuilt =
            rebuild::<CreateGuildWelcomeChannelDe, CreateGuildWelcomeChannel<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn role_colour_boundaries_roundtrip() {
    let cases: Vec<Value> = vec![
        json!({ "primary_color": 0 }),
        json!({ "primary_color": 16777215u32 }),
        json!({
            "primary_color": 0,
            "secondary_color": 0,
            "tertiary_color": 16777215u32,
        }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateRoleColoursDe, CreateRoleColours>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn test_entitlement_id_boundaries_roundtrip() {
    // Non-max snowflakes: u64::MAX itself is not representable upstream.
    let payload = json!({
        "sku_id": "18446744073709551614",
        "owner_id": "18446744073709551614",
        "owner_type": 2,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateTestEntitlementDe, CreateTestEntitlement>(&payload);

    assert_value_eq(&expected, &rebuilt);
}
