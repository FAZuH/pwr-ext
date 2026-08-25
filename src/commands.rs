//! Wrapper types for serenity's application-command builders.

use std::borrow::Cow;
use std::collections::HashMap;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as _;
use serde_json::Value;
use serenity::builder::CreateCommand;
use serenity::builder::CreateCommandOption;
use serenity::builder::CreateCommandPermission;
use serenity::model::application::CommandOptionType;
use serenity::model::application::CommandPermissionType;
use serenity::model::application::CommandType;
use serenity::model::application::EntryPointHandlerType;
use serenity::model::application::InstallationContext;
use serenity::model::application::InteractionContext;
use serenity::model::channel::ChannelType;
use serenity::model::id::CommandPermissionId;

use crate::util::mirror;
use crate::util::opaque_wrapper;

/// Mirror of [`CreateCommand`].
///
/// Upstream flattens an `EditCommand` into the emitted JSON; the mirror
/// reproduces that flat shape directly. Fields upstream always serializes
/// (`name_localizations`, `description_localizations`, `options`, `nsfw`)
/// default leniently and rebuild byte-stable; a missing `name` rebuilds as
/// the empty string because upstream offers no way back to `null`.
#[derive(Debug, Deserialize)]
pub struct CreateCommandDe {
    #[serde(rename = "type", default)]
    kind: Option<CommandType>,
    #[serde(default)]
    handler: Option<EntryPointHandlerType>,
    name: Option<Cow<'static, str>>,
    #[serde(default)]
    name_localizations: HashMap<Cow<'static, str>, Cow<'static, str>>,
    description: Option<Cow<'static, str>>,
    #[serde(default)]
    description_localizations: HashMap<Cow<'static, str>, Cow<'static, str>>,
    #[serde(default)]
    options: Vec<CreateCommandOptionDe>,
    #[serde(default)]
    default_member_permissions: Option<serenity::model::Permissions>,
    dm_permission: Option<bool>,
    integration_types: Option<Vec<InstallationContext>>,
    contexts: Option<Vec<InteractionContext>>,
    nsfw: bool,
}

impl From<CreateCommandDe> for CreateCommand<'static> {
    fn from(de: CreateCommandDe) -> Self {
        let mut command = CreateCommand::new(de.name.unwrap_or_default());
        if let Some(kind) = de.kind {
            command = command.kind(kind);
        }
        if let Some(handler) = de.handler {
            command = command.handler(handler);
        }
        for (locale, name) in de.name_localizations {
            command = command.name_localized(locale, name);
        }
        if let Some(description) = de.description {
            command = command.description(description);
        }
        for (locale, description) in de.description_localizations {
            command = command.description_localized(locale, description);
        }
        if !de.options.is_empty() {
            command = command.set_options(
                de.options
                    .into_iter()
                    .map(CreateCommandOption::from)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(permissions) = de.default_member_permissions {
            command = command.default_member_permissions(permissions);
        }
        if let Some(dm_permission) = de.dm_permission {
            command = command.dm_permission(dm_permission);
        }
        if let Some(integration_types) = de.integration_types {
            command = command.integration_types(std::borrow::Cow::Owned(integration_types));
        }
        if let Some(contexts) = de.contexts {
            command = command.contexts(std::borrow::Cow::Owned(contexts));
        }
        command.nsfw(de.nsfw)
    }
}

/// Mirror of [`CreateCommandOption`], recursive over its own `options`.
///
/// Every scalar/list field upstream emits unconditionally (with explicit
/// `null`s for unset bounds); only `file_types` is skipped when empty.
#[derive(Debug, Deserialize)]
pub struct CreateCommandOptionDe {
    #[serde(rename = "type")]
    kind: CommandOptionType,
    name: Cow<'static, str>,
    #[serde(default)]
    name_localizations: Option<HashMap<Cow<'static, str>, Cow<'static, str>>>,
    description: Cow<'static, str>,
    #[serde(default)]
    description_localizations: Option<HashMap<Cow<'static, str>, Cow<'static, str>>>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    choices: Vec<CommandOptionChoiceDe>,
    #[serde(default)]
    options: Vec<CreateCommandOptionDe>,
    #[serde(default)]
    channel_types: Vec<ChannelType>,
    #[serde(default)]
    min_value: Option<serde_json::Number>,
    #[serde(default)]
    max_value: Option<serde_json::Number>,
    #[serde(default)]
    min_length: Option<u16>,
    #[serde(default)]
    max_length: Option<u16>,
    #[serde(default)]
    file_types: Vec<Cow<'static, str>>,
    #[serde(default)]
    autocomplete: bool,
}

impl From<CreateCommandOptionDe> for CreateCommandOption<'static> {
    fn from(de: CreateCommandOptionDe) -> Self {
        let mut option = CreateCommandOption::new(de.kind, de.name.clone(), de.description.clone());
        if let Some(localizations) = de.name_localizations {
            for (locale, name) in localizations {
                option = option.name_localized(locale, name);
            }
        }
        if let Some(localizations) = de.description_localizations {
            for (locale, description) in localizations {
                option = option.description_localized(locale, description);
            }
        }
        option = option.required(de.required);
        for choice in de.choices {
            option = choice.add_to(option);
        }
        if !de.options.is_empty() {
            option = option.set_sub_options(
                de.options
                    .into_iter()
                    .map(CreateCommandOption::from)
                    .collect::<Vec<_>>(),
            );
        }
        if !de.channel_types.is_empty() {
            option = option.channel_types(de.channel_types);
        }
        // The int/number/string setter split exists only on the Rust side;
        // the wire value decides which one reproduces the same JSON.
        if let Some(min_value) = de.min_value {
            option = set_min_value(option, &min_value);
        }
        if let Some(max_value) = de.max_value {
            option = set_max_value(option, &max_value);
        }
        if let Some(min_length) = de.min_length {
            option = option.min_length(min_length);
        }
        if let Some(max_length) = de.max_length {
            option = option.max_length(max_length);
        }
        if !de.file_types.is_empty() {
            option = option.file_types(de.file_types);
        }
        option.set_autocomplete(de.autocomplete)
    }
}

fn set_min_value(
    option: CreateCommandOption<'static>,
    value: &serde_json::Number,
) -> CreateCommandOption<'static> {
    if let Some(int_value) = value.as_i64() {
        option.min_int_value(int_value)
    } else {
        let float_value = value.as_f64().expect("JSON number converts to f64");
        option.min_number_value(float_value)
    }
}

fn set_max_value(
    option: CreateCommandOption<'static>,
    value: &serde_json::Number,
) -> CreateCommandOption<'static> {
    if let Some(int_value) = value.as_i64() {
        option.max_int_value(int_value)
    } else {
        let float_value = value.as_f64().expect("JSON number converts to f64");
        option.max_number_value(float_value)
    }
}

/// Mirror of the upstream-private [`CreateCommandOptionChoice`].
///
/// Choice values are plain JSON on the wire; rebuilding dispatches on the
/// value kind to the matching typed setter (int/string/number), which
/// reproduces identical JSON. Upstream's builders can only produce string
/// or number values, so anything else errors at deserialization time.
#[derive(Debug)]
struct CommandOptionChoiceDe {
    name: Cow<'static, str>,
    name_localizations: Option<HashMap<Cow<'static, str>, Cow<'static, str>>>,
    value: Value,
}

impl<'de> Deserialize<'de> for CommandOptionChoiceDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            name: Cow<'static, str>,
            name_localizations: Option<HashMap<Cow<'static, str>, Cow<'static, str>>>,
            value: Value,
        }

        let raw = Raw::deserialize(deserializer)?;
        match raw.value {
            Value::String(_) | Value::Number(_) => Ok(Self {
                name: raw.name,
                name_localizations: raw.name_localizations,
                value: raw.value,
            }),
            other => Err(D::Error::custom(format!(
                "choice values must be a string or a number, got {other}"
            ))),
        }
    }
}

impl CommandOptionChoiceDe {
    fn add_to(self, option: CreateCommandOption<'static>) -> CreateCommandOption<'static> {
        match self.value {
            Value::String(string) => match self.name_localizations {
                Some(locales) => option.add_string_choice_localized(self.name, string, locales),
                None => option.add_string_choice(self.name, string),
            },
            Value::Number(number) => {
                if let Some(int_value) = number.as_i64() {
                    match self.name_localizations {
                        Some(locales) => {
                            option.add_int_choice_localized(self.name, int_value, locales)
                        }
                        None => option.add_int_choice(self.name, int_value),
                    }
                } else {
                    let float_value = number
                        .as_f64()
                        .expect("non-integer JSON numbers convert to f64");
                    match self.name_localizations {
                        Some(locales) => {
                            option.add_number_choice_localized(self.name, float_value, locales)
                        }
                        None => option.add_number_choice(self.name, float_value),
                    }
                }
            }
            other => unreachable!("rejected during deserialization: {other}"),
        }
    }
}

/// Mirror of [`CreateCommandPermission`], rebuilt through its typed
/// constructors.
///
/// The wire shape is `{id, type, permission}` with `type` 1 = role,
/// 2 = user, 3 = channel. Upstream exposes no constructor for unknown
/// kinds, so any other number errors at deserialization time (spec D5).
#[derive(Debug)]
pub struct CreateCommandPermissionDe(CreateCommandPermission);

opaque_wrapper!(
    CreateCommandPermissionDe,
    CreateCommandPermission,
    parse_command_permission
);

#[derive(Debug, Deserialize)]
struct RawCommandPermissionDe {
    id: CommandPermissionId,
    permission: bool,
}

fn parse_command_permission(value: &Value) -> Result<CreateCommandPermission, String> {
    #[derive(Deserialize)]
    struct Tagged {
        #[serde(rename = "type")]
        kind: CommandPermissionType,
    }

    let raw: RawCommandPermissionDe = mirror(value)?;
    let tagged: Tagged = mirror(value)?;
    Ok(match tagged.kind {
        CommandPermissionType::Role => CreateCommandPermission::role(
            serenity::model::id::RoleId::new(raw.id.get()),
            raw.permission,
        ),
        CommandPermissionType::User => CreateCommandPermission::user(
            serenity::model::id::UserId::new(raw.id.get()),
            raw.permission,
        ),
        CommandPermissionType::Channel => CreateCommandPermission::channel(
            serenity::model::id::ChannelId::new(raw.id.get()),
            raw.permission,
        ),
        other => {
            return Err(format!("unsupported command permission type {other:?}"));
        }
    })
}
