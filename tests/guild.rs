//! Round-trip suite for the guild request builders: channels (incl. forum
//! tags), forum posts, threads, invites, scheduled events, stage instances,
//! and webhooks.

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateChannel;
use serenity::builder::CreateForumPost;
use serenity::builder::CreateForumTag;
use serenity::builder::CreateInvite;
use serenity::builder::CreateMessage;
use serenity::builder::CreateScheduledEvent;
use serenity::builder::CreateStageInstance;
use serenity::builder::CreateThread;
use serenity::builder::CreateWebhook;
use serenity::model::Permissions;
use serenity::model::channel::AutoArchiveDuration;
use serenity::model::channel::ChannelFlags;
use serenity::model::channel::ChannelType;
use serenity::model::channel::ForumEmoji;
use serenity::model::channel::ForumLayoutType;
use serenity::model::channel::PermissionOverwrite;
use serenity::model::channel::PermissionOverwriteType;
use serenity::model::channel::SortOrder;
use serenity::model::channel::VideoQualityMode;
use serenity::model::guild::ScheduledEventType;
use serenity::model::id::ApplicationId;
use serenity::model::id::ChannelId;
use serenity::model::id::EmojiId;
use serenity::model::id::ForumTagId;
use serenity::model::id::RoleId;
use serenity::model::invite::InviteTargetType;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

// ---- CreateChannel ----

#[test]
fn text_channel_with_every_field_set_roundtrips() {
    let channel = CreateChannel::new("announcements")
        .kind(ChannelType::Text)
        .topic("Server-wide announcements")
        .nsfw(false)
        .position(3)
        .rate_limit_per_user(serenity::nonmax::NonMaxU16::new(10).expect("not u16::MAX"))
        .category(ChannelId::new(482_842_547_397_828_608))
        .permissions(vec![PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::SEND_MESSAGES,
            kind: PermissionOverwriteType::Role(RoleId::new(182_894_738_100_322_304)),
        }])
        .flags(ChannelFlags::PINNED);

    assert_roundtrip::<CreateChannelDe, _>(&channel);
}

#[test]
fn voice_channel_with_media_fields_roundtrips() {
    let channel = CreateChannel::new("Game night")
        .kind(ChannelType::Voice)
        .bitrate(96_000)
        .user_limit(serenity::nonmax::NonMaxU16::new(12).expect("not u16::MAX"))
        .rtc_region(std::borrow::Cow::Borrowed("europe"))
        .video_quality_mode(VideoQualityMode::Full);

    assert_roundtrip::<CreateChannelDe, _>(&channel);
}

#[test]
fn forum_channel_with_tags_and_defaults_roundtrips() {
    let channel = CreateChannel::new("help-desk")
        .kind(ChannelType::Forum)
        .default_auto_archive_duration(AutoArchiveDuration::OneWeek)
        .default_reaction_emoji(ForumEmoji::Name("🔥".to_owned().try_into().expect("fits")))
        .available_tags(vec![
            CreateForumTag::new("answered").moderated(true),
            CreateForumTag::new("urgent"),
            CreateForumTag::new("bug").emoji(EmojiId::new(936_926_320_553_408_522)),
            CreateForumTag::new("🔥"),
        ])
        .default_sort_order(SortOrder::CreationDate)
        .default_forum_layout(ForumLayoutType::GalleryView)
        .default_thread_rate_limit_per_user(
            serenity::nonmax::NonMaxU16::new(60).expect("not u16::MAX"),
        );

    assert_roundtrip::<CreateChannelDe, _>(&channel);
}

#[test]
fn minimal_text_channel_roundtrips() {
    let channel = CreateChannel::new("general");

    assert_roundtrip::<CreateChannelDe, _>(&channel);
}

// ---- CreateForumPost ----

#[test]
fn forum_post_with_message_and_applied_tags_roundtrips() {
    let post = CreateForumPost::new(
        "Latency spikes in eu-central",
        CreateMessage::new().content("**Incident #4021** — investigating"),
    )
    .auto_archive_duration(AutoArchiveDuration::OneDay)
    .set_applied_tags(vec![ForumTagId::new(7), ForumTagId::new(12)]);

    assert_roundtrip::<CreateForumPostDe, _>(&post);
}

#[test]
fn minimal_forum_post_roundtrips() {
    let post = CreateForumPost::new("Intro", CreateMessage::new());

    assert_roundtrip::<CreateForumPostDe, _>(&post);
}

// ---- CreateThread ----

#[test]
fn private_thread_with_all_fields_roundtrips() {
    let thread = CreateThread::new("Moderation log")
        .auto_archive_duration(AutoArchiveDuration::ThreeDays)
        .kind(ChannelType::PrivateThread)
        .invitable(true)
        .rate_limit_per_user(serenity::nonmax::NonMaxU16::new(5).expect("not u16::MAX"));

    assert_roundtrip::<CreateThreadDe, _>(&thread);
}

#[test]
fn public_thread_minimal_roundtrips() {
    let thread = CreateThread::new("Weekly planning");

    assert_roundtrip::<CreateThreadDe, _>(&thread);
}

// ---- CreateInvite ----

#[test]
fn invite_with_every_field_set_roundtrips() {
    let invite = CreateInvite::new()
        .max_age(3600)
        .max_uses(10)
        .temporary(true)
        .unique(false)
        .target_type(InviteTargetType::EmbeddedApplication)
        .target_application_id(ApplicationId::new(936_926_320_553_408_522))
        .role_ids(vec![
            RoleId::new(182_894_738_100_322_304),
            RoleId::new(110_372_470_472_613_888),
        ]);

    assert_roundtrip::<CreateInviteDe, _>(&invite);
}

#[test]
fn default_invite_roundtrips() {
    let invite = CreateInvite::new();

    assert_roundtrip::<CreateInviteDe, _>(&invite);
}

#[test]
fn invite_empty_role_ids_stay_explicit() {
    // Option<Vec<_>> distinguishes an empty array from an absent key.
    let payload = json!({
        "role_ids": [],
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateInviteDe, CreateInvite<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- CreateScheduledEvent ----

#[test]
fn external_event_with_metadata_roundtrips() {
    let event = CreateScheduledEvent::new(
        ScheduledEventType::External,
        "Release party 2026.34",
        "2026-09-01T18:00:00Z"
            .parse::<serenity::model::Timestamp>()
            .expect("valid RFC 3339"),
    )
    .location("#release-party stage")
    .end_time(
        "2026-09-01T20:00:00Z"
            .parse::<serenity::model::Timestamp>()
            .expect("valid RFC 3339"),
    )
    .description("Celebrating the release train");

    assert_roundtrip::<CreateScheduledEventDe, _>(&event);
}

#[test]
fn stage_event_in_channel_roundtrips() {
    let event = CreateScheduledEvent::new(
        ScheduledEventType::StageInstance,
        "Ask me anything",
        "2026-09-15T19:30:00Z"
            .parse::<serenity::model::Timestamp>()
            .expect("valid RFC 3339"),
    )
    .channel_id(ChannelId::new(482_842_547_397_828_608));

    assert_roundtrip::<CreateScheduledEventDe, _>(&event);
}

#[test]
fn scheduled_event_privacy_level_normalizes_to_guild_only() {
    // Upstream pins privacy_level to GuildOnly in its constructor and
    // exposes no setter; any other wire value normalizes to 2.
    let payload = json!({
        "name": "Mystery event",
        "privacy_level": 1,
        "scheduled_start_time": "2026-09-01T18:00:00Z",
        "entity_type": 2,
    });
    let expected = json!({
        "name": "Mystery event",
        "privacy_level": 2,
        "scheduled_start_time": "2026-09-01T18:00:00Z",
        "entity_type": 2,
    });

    let rebuilt = rebuild::<CreateScheduledEventDe, CreateScheduledEvent<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn event_metadata_without_location_normalizes_to_absent() {
    // A metadata object carrying no location cannot be expressed through
    // the public setters; it rebuilds as no metadata at all.
    let payload = json!({
        "channel_id": "482842547397828608",
        "entity_metadata": { "location": null },
        "name": "Voice hangout",
        "privacy_level": 2,
        "scheduled_start_time": "2026-09-02T20:00:00Z",
        "entity_type": 2,
    });
    let expected = json!({
        "channel_id": "482842547397828608",
        "name": "Voice hangout",
        "privacy_level": 2,
        "scheduled_start_time": "2026-09-02T20:00:00Z",
        "entity_type": 2,
    });

    let rebuilt = rebuild::<CreateScheduledEventDe, CreateScheduledEvent<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- CreateStageInstance ----

#[test]
fn stage_instance_with_notification_roundtrips() {
    let stage = CreateStageInstance::new("Weekly standup").send_start_notification(true);

    assert_roundtrip::<CreateStageInstanceDe, _>(&stage);
}

#[test]
fn stage_instance_channel_id_is_accepted_but_dropped_on_rebuild() {
    // Upstream fills channel_id only inside its HTTP executor; the public
    // builder has no setter, so the wire value rebuilds as null.
    let payload = json!({
        "channel_id": "555",
        "topic": "Standup notes",
        "privacy_level": 2,
    });
    let expected = json!({
        "channel_id": null,
        "topic": "Standup notes",
        "privacy_level": 2,
    });

    let rebuilt = rebuild::<CreateStageInstanceDe, CreateStageInstance<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn stage_instance_privacy_level_normalizes_to_guild_only() {
    // Upstream fills channel_id only inside its HTTP executor; the public
    // builder has no setter, so the wire value rebuilds as null.
    let payload = json!({
        "topic": "Deprecated public stage",
        "privacy_level": 1,
    });
    let expected = json!({
        "channel_id": null,
        "topic": "Deprecated public stage",
        "privacy_level": 2,
    });

    let rebuilt = rebuild::<CreateStageInstanceDe, CreateStageInstance<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- CreateWebhook ----

#[test]
fn webhook_with_avatar_roundtrips() {
    let webhook = CreateWebhook::new("Deploy bot").avatar(
        serenity::builder::DataUri::from_base64(
            "data:image/png;base64,R0lGODlhAQABAIAAAP///wAAACwAAAAAAQABAAACAkQBADs=",
        )
        .expect("valid data URI"),
    );

    assert_roundtrip::<CreateWebhookDe, _>(&webhook);
}

#[test]
fn minimal_webhook_roundtrips() {
    let webhook = CreateWebhook::new("CI reporter");

    assert_roundtrip::<CreateWebhookDe, _>(&webhook);
}

#[test]
fn webhook_avatar_invalid_data_uri_errors() {
    let payload = json!({
        "name": "broken bot",
        "avatar": "https://cdn.example.com/avatar.png",
    });

    let result: Result<CreateWebhookDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "an http URL is not a data URI");
}

// ---- Error paths ----

#[test]
fn forum_tag_cannot_carry_both_emojis_errors() {
    let payload = json!({
        "name": "doubled",
        "moderated": false,
        "emoji_id": "42",
        "emoji_name": "🔥",
    });

    let result: Result<CreateForumTagDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "both emoji fields together are not representable upstream"
    );
}

#[test]
fn scheduled_event_without_entity_type_errors() {
    let payload = json!({
        "name": "No type",
        "privacy_level": 2,
        "scheduled_start_time": "2026-09-01T18:00:00Z",
    });

    let result: Result<CreateScheduledEventDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "\"entity_type\" is always serialized upstream"
    );
}

#[test]
fn thread_without_name_errors() {
    let payload = json!({
        "type": 11,
    });

    let result: Result<CreateThreadDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "\"name\" is required on threads");
}

// ---- Boundary values ----

#[test]
fn channel_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        // Empty strings and zero-valued numerics.
        json!({
            "name": "",
            "type": 0,
            "topic": "",
            "bitrate": 0,
            "user_limit": 0,
            "rate_limit_per_user": 0,
            "position": 0,
        }),
        // NonMaxU16 upper bound: 65534 (65535 is not representable).
        json!({
            "name": "capped",
            "type": 2,
            "user_limit": 65534,
            "default_thread_rate_limit_per_user": 65534,
        }),
        // Unknown numeric tags survive where upstream has Unknown(u8)
        // fallbacks (spec D5).
        json!({ "name": "future", "type": 99 }),
        json!({ "name": "future forum", "type": 15, "default_sort_order": 9, "default_forum_layout": 9 }),
        json!({
            "name": "future media",
            "type": 13,
            "video_quality_mode": 9,
            "default_auto_archive_duration": 500,
        }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateChannelDe, CreateChannel<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn channel_unknown_flag_bits_truncate_like_upstream() {
    // ChannelFlags is u32-backed; only bits 1, 4, and 21 are defined.
    let payload = json!({
        "name": "pinned",
        "type": 0,
        "flags": 65535u16,
    });
    let expected = json!({
        "name": "pinned",
        "type": 0,
        "flags": 18u16,
    });

    let rebuilt = rebuild::<CreateChannelDe, CreateChannel<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn thread_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        json!({ "name": "" }),
        // Unknown thread type tag (spec D5).
        json!({ "name": "mystery", "type": 77, "invitable": false }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateThreadDe, CreateThread<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn invite_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        json!({ "max_age": 0, "max_uses": 0, "temporary": false, "unique": true }),
        // Unknown target type number (spec D5).
        json!({ "target_type": 9 }),
        json!({ "role_ids": [] }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateInviteDe, CreateInvite<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}
