//! Round-trip suite for the application-command family: `CreateCommand`
//! (flattened wire shape), the recursive `CreateCommandOption` tree with its
//! private choice mirror, and command permissions.

use std::borrow::Cow;
use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;
use serenity::builder::CreateCommand;
use serenity::builder::CreateCommandOption;
use serenity::builder::CreateCommandPermission;
use serenity::model::Permissions;
use serenity::model::application::CommandOptionType;
use serenity::model::application::CommandType;
use serenity::model::application::EntryPointHandlerType;
use serenity::model::application::InstallationContext;
use serenity::model::application::InteractionContext;
use serenity::model::channel::ChannelType;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::id::RoleId;
use serenity::model::id::UserId;

#[path = "roundtrip_helpers.rs"]
mod helpers;

use helpers::assert_roundtrip;
use helpers::assert_value_eq;
use helpers::rebuild;
use pwr_ext::prelude::*;

fn localized_pair() -> HashMap<Cow<'static, str>, Cow<'static, str>> {
    HashMap::from([
        (Cow::Borrowed("zh-CN"), Cow::Borrowed("生日")),
        (Cow::Borrowed("el"), Cow::Borrowed("γενέθλια")),
    ])
}

// ---- CreateCommand ----

#[test]
fn command_with_every_field_set_roundtrips() {
    let command = CreateCommand::new("birthday")
        .kind(CommandType::ChatInput)
        .description("Wish a friend a happy birthday")
        .default_member_permissions(Permissions::MODERATE_MEMBERS)
        .dm_permission(false)
        .add_integration_type(InstallationContext::Guild)
        .add_integration_type(InstallationContext::User)
        .add_context(InteractionContext::BotDm)
        .nsfw(true)
        .set_options(vec![
            CreateCommandOption::new(CommandOptionType::String, "friend", "The friend to greet")
                .required(true),
            CreateCommandOption::new(CommandOptionType::Integer, "age", "Their age")
                .add_int_choice("one", 1)
                .min_int_value(0)
                .max_int_value(150),
        ]);

    assert_roundtrip::<CreateCommandDe, _>(&command);
}

#[test]
fn command_localizations_roundtrip() {
    let command = CreateCommand::new("birthday")
        .name_localized("zh-CN", "生日")
        .name_localized("el", "γενέθλια")
        .description("Wish a friend a happy birthday")
        .description_localized("zh-CN", "祝你朋友生日快乐");

    assert_roundtrip::<CreateCommandDe, _>(&command);
}

#[test]
fn minimal_command_roundtrips() {
    let command = CreateCommand::new("ping");

    assert_roundtrip::<CreateCommandDe, _>(&command);
}

#[test]
fn primary_entry_point_command_with_handler_roundtrips() {
    let command = CreateCommand::new("my-app")
        .kind(CommandType::PrimaryEntryPoint)
        .handler(EntryPointHandlerType::DiscordLaunchActivity);

    assert_roundtrip::<CreateCommandDe, _>(&command);
}

#[test]
fn recursive_subcommand_tree_roundtrips() {
    let command = CreateCommand::new("mod").set_options(vec![
        CreateCommandOption::new(CommandOptionType::SubCommandGroup, "ban", "Ban management")
            .set_sub_options(vec![
                CreateCommandOption::new(CommandOptionType::SubCommand, "add", "Ban a member")
                    .set_sub_options(vec![
                        CreateCommandOption::new(CommandOptionType::User, "member", "Who to ban")
                            .required(true),
                        CreateCommandOption::new(CommandOptionType::String, "reason", "Why to ban"),
                    ]),
            ]),
        CreateCommandOption::new(CommandOptionType::SubCommand, "kick", "Kick a member"),
    ]);

    assert_roundtrip::<CreateCommandDe, _>(&command);
}

// ---- CreateCommandOption ----

#[test]
fn option_kinds_with_typed_bounds_roundtrip() {
    let integer_option = CreateCommandOption::new(CommandOptionType::Integer, "level", "Level")
        .min_int_value(-5)
        .max_int_value(100);
    let number_option = CreateCommandOption::new(CommandOptionType::Number, "ratio", "Ratio")
        .min_number_value(-1.5)
        .max_number_value(2.75);
    let string_option = CreateCommandOption::new(CommandOptionType::String, "nick", "Nickname")
        .min_length(1)
        .max_length(32);
    let channel_option = CreateCommandOption::new(CommandOptionType::Channel, "target", "Where")
        .channel_types(vec![ChannelType::Text, ChannelType::Voice]);
    let attachment_option =
        CreateCommandOption::new(CommandOptionType::Attachment, "log", "Log file")
            .file_types(vec![Cow::Borrowed(".txt"), Cow::Borrowed("image")]);

    assert_roundtrip::<CreateCommandOptionDe, _>(&integer_option);
    assert_roundtrip::<CreateCommandOptionDe, _>(&number_option);
    assert_roundtrip::<CreateCommandOptionDe, _>(&string_option);
    assert_roundtrip::<CreateCommandOptionDe, _>(&channel_option);
    assert_roundtrip::<CreateCommandOptionDe, _>(&attachment_option);
}

#[test]
fn choice_values_dispatch_to_matching_setters() {
    // int/string/number choices serialize identically on the wire; the
    // value kind decides which typed setter reproduces them.
    let payload = json!({
        "type": 3,
        "name": "difficulty",
        "description": "How hard",
        "required": false,
        "choices": [
            { "name": "Easy", "value": 1 },
            { "name": "Precise", "value": 1.5 },
            { "name": "Label", "value": "hard" },
            { "name": "Negative", "value": -3, "name_localizations": { "de-DE": "Schwer" } },
        ],
        "options": [],
        "channel_types": [],
        "min_value": null,
        "max_value": null,
        "min_length": null,
        "max_length": null,
        "autocomplete": false,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateCommandOptionDe, CreateCommandOption<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn localized_choices_roundtrip() {
    let option = CreateCommandOption::new(CommandOptionType::Integer, "tier", "Tier")
        .add_int_choice_localized("low", 1, localized_pair());

    assert_roundtrip::<CreateCommandOptionDe, _>(&option);
}

#[test]
fn unknown_option_type_tag_survives_the_roundtrip() {
    // CommandOptionType falls back to Unknown(u8) upstream (spec D5).
    let payload = json!({
        "type": 99,
        "name": "mystery",
        "description": "From the future",
        "required": false,
        "autocomplete": false,
    });

    let expected = json!({
        "type": 99,
        "name": "mystery",
        "description": "From the future",
        "required": false,
        "choices": [],
        "options": [],
        "channel_types": [],
        "min_value": null,
        "max_value": null,
        "min_length": null,
        "max_length": null,
        "autocomplete": false,
    });

    let rebuilt = rebuild::<CreateCommandOptionDe, CreateCommandOption<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- Documented canonicalizations ----

#[test]
fn unset_command_name_rebuilds_as_empty_string() {
    // Upstream serializes an unset name as null but offers no way back to
    // null once built; rebuilding canonically fills the empty string.
    let payload = json!({
        "name": null,
        "name_localizations": {},
        "description_localizations": {},
        "options": [],
        "nsfw": false,
    });
    let expected = json!({
        "name": "",
        "name_localizations": {},
        "description_localizations": {},
        "options": [],
        "nsfw": false,
    });

    let rebuilt = rebuild::<CreateCommandDe, CreateCommand<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

#[test]
fn option_unset_scalars_rebuild_byte_stable() {
    // Upstream always emits these fields; explicit nulls pass through.
    let payload = json!({
        "type": 4,
        "name": "count",
        "description": "How many",
        "required": false,
        "choices": [],
        "options": [],
        "channel_types": [],
        "min_value": null,
        "max_value": null,
        "min_length": null,
        "max_length": null,
        "autocomplete": false,
    });
    let expected = payload.clone();

    let rebuilt = rebuild::<CreateCommandOptionDe, CreateCommandOption<'static>>(&payload);

    assert_value_eq(&expected, &rebuilt);
}

// ---- Error paths ----

#[test]
fn choice_value_of_wrong_json_kind_errors() {
    let values: Vec<Value> = vec![json!(true), json!([1]), json!({ "n": 1 }), Value::Null];
    for value in values {
        let payload = json!({
            "type": 3,
            "name": "bad",
            "description": "Bad choices",
            "choices": [{ "name": "broken", "value": value }],
        });

        let result: Result<CreateCommandOptionDe, _> = serde_json::from_value(payload);

        assert!(
            result.is_err(),
            "a {value} choice value is not representable upstream"
        );
    }
}

#[test]
fn command_permission_unknown_kind_errors() {
    let payload = json!({
        "id": "42",
        "type": 7,
        "permission": true,
    });

    let result: Result<CreateCommandPermissionDe, _> = serde_json::from_value(payload);

    assert!(
        result.is_err(),
        "upstream has no constructor for unknown permission kinds"
    );
}

#[test]
fn command_without_nsfw_errors_on_flattened_shape() {
    // nsfw serializes unconditionally on the flattened shape; name is an
    // Option and tolerates absence, but nsfw must not be dropped.
    let payload = json!({
        "name_localizations": {},
        "description_localizations": {},
        "options": [],
    });

    let result: Result<CreateCommandDe, _> = serde_json::from_value(payload);

    assert!(result.is_err(), "\"nsfw\" is always serialized upstream");
}

// ---- Boundary values ----

#[test]
fn command_boundary_values_roundtrip() {
    let cases: Vec<Value> = vec![
        // Permissions serialize back as strings; zero and a high
        // defined bit (MODERATE_MEMBERS = 1 << 40).
        json!({
            "name": "perms",
            "name_localizations": {},
            "description_localizations": {},
            "options": [],
            "default_member_permissions": "0",
            "nsfw": false,
        }),
        json!({
            "name": "perms-high",
            "name_localizations": {},
            "description_localizations": {},
            "options": [],
            "default_member_permissions": "1099511627776",
            "nsfw": true,
        }),
        // Empty localizations maps and empty strings.
        json!({
            "name": "",
            "name_localizations": {},
            "description": "",
            "description_localizations": {},
            "options": [],
            "nsfw": false,
            "integration_types": [9],
            "contexts": [7],
        }),
    ];

    for payload in cases {
        let rebuilt = rebuild::<CreateCommandDe, CreateCommand<'static>>(&payload);
        assert_value_eq(&payload, &rebuilt);
    }
}

#[test]
fn every_command_permission_kind_roundtrips() {
    let role = CreateCommandPermission::role(RoleId::new(182_894_738_100_322_304), true);
    let user = CreateCommandPermission::user(UserId::new(110_372_470_472_613_888), false);
    let channel = CreateCommandPermission::channel(ChannelId::new(482_842_547_397_828_608), true);
    let everyone = CreateCommandPermission::everyone(GuildId::new(902_361_593_214_052_352), false);

    assert_roundtrip::<CreateCommandPermissionDe, _>(&role);
    assert_roundtrip::<CreateCommandPermissionDe, _>(&user);
    assert_roundtrip::<CreateCommandPermissionDe, _>(&channel);
    assert_roundtrip::<CreateCommandPermissionDe, _>(&everyone);

    // The all-channels id is guild_id - 1 on the wire; it round-trips
    // through the plain channel constructor with identical JSON.
    let all_channels =
        CreateCommandPermission::all_channels(GuildId::new(902_361_593_214_052_352), true);
    assert_roundtrip::<CreateCommandPermissionDe, _>(&all_channels);
}

#[test]
fn command_permissions_wire_array_roundtrips() {
    let permissions = vec![
        CreateCommandPermission::role(RoleId::new(1), true),
        CreateCommandPermission::user(UserId::new(2), false),
        CreateCommandPermission::channel(ChannelId::new(3), true),
    ];

    // Sanity-check the wire shape against raw JSON once, then rely on the
    // builder-level tests above.
    let expected = json!([
        { "id": "1", "type": 1, "permission": true },
        { "id": "2", "type": 2, "permission": false },
        { "id": "3", "type": 3, "permission": true },
    ]);
    let actual = serde_json::to_value(&permissions).expect("serializes");
    assert_value_eq(&expected, &actual);
}
