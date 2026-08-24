//! Wrapper types for serenity's interaction-response builders.
//!
//! The scope exclusion for this family was lifted on 2026-08-23: every
//! builder here emits plain JSON, and each nests mirrors that already exist
//! in this crate (embeds, components, allowed mentions, polls, modals).

use std::borrow::Cow;
use std::collections::HashMap;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as _;
use serde_json::Value;
use serenity::builder::AutocompleteChoice;
use serenity::builder::AutocompleteValue;
use serenity::builder::CreateAutocompleteResponse;
use serenity::builder::CreateInteractionResponse;
use serenity::builder::CreateInteractionResponseFollowup;
use serenity::builder::CreateInteractionResponseMessage;

use crate::components::CreateComponentDe;
use crate::embed::CreateEmbedDe;
use crate::message::CreateAllowedMentionsDe;
use crate::poll::CreatePollDe;
use crate::util::capture_value;
use crate::util::mirror;

/// Mirror of the upstream enum [`CreateInteractionResponse`].
///
/// Upstream serializes by hand as `{"type": <kind>, "data": <payload|null>}`;
/// the mirror dispatches on the numeric kind tag. The upstream enum has no
/// unknown-value fallback, so any other kind errors at deserialization time.
#[derive(Debug)]
pub struct CreateInteractionResponseDe(pub CreateInteractionResponse<'static>);

impl From<CreateInteractionResponseDe> for CreateInteractionResponse<'static> {
    fn from(de: CreateInteractionResponseDe) -> Self {
        de.0
    }
}

impl<'de> Deserialize<'de> for CreateInteractionResponseDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = capture_value(deserializer)?;
        parse_interaction_response(&value)
            .map_err(D::Error::custom)
            .map(CreateInteractionResponseDe)
    }
}

fn parse_interaction_response(value: &Value) -> Result<CreateInteractionResponse<'static>, String> {
    let kind = value
        .get("type")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("interaction response missing numeric \"type\": {value}"))?;
    let data = value
        .get("data")
        .ok_or_else(|| format!("interaction response missing \"data\": {value}"))?;

    Ok(match kind {
        1 => CreateInteractionResponse::Pong,
        4 => CreateInteractionResponse::Message(parse_response_message(data)?),
        5 => CreateInteractionResponse::Defer(parse_response_message(data)?),
        6 => CreateInteractionResponse::Acknowledge,
        7 => CreateInteractionResponse::UpdateMessage(parse_response_message(data)?),
        8 => CreateInteractionResponse::Autocomplete(parse_autocomplete(data)?),
        9 => {
            let modal: crate::modal::CreateModalDe =
                mirror(data).map_err(|_| format!("invalid modal payload: {data}"))?;
            CreateInteractionResponse::Modal(modal.into())
        }
        12 => CreateInteractionResponse::LaunchActivity,
        other => {
            return Err(format!(
                "unsupported interaction response kind {other}: {value}"
            ));
        }
    })
}

/// Mirror of [`CreateInteractionResponseMessage`].
///
/// The `attachments` array is part of the multipart plumbing excluded from
/// round-trip support; rebuilt messages always emit an empty array.
#[derive(Debug, Deserialize)]
pub struct CreateInteractionResponseMessageDe {
    #[serde(default)]
    tts: Option<bool>,
    #[serde(default)]
    content: Option<Cow<'static, str>>,
    #[serde(default)]
    embeds: Option<Vec<CreateEmbedDe<'static>>>,
    allowed_mentions: Option<CreateAllowedMentionsDe>,
    flags: Option<serenity::model::channel::MessageFlags>,
    components: Option<Vec<CreateComponentDe>>,
    poll: Option<CreatePollDe>,
    #[serde(rename = "attachments", default)]
    _attachments: Vec<Value>,
}

impl From<CreateInteractionResponseMessageDe> for CreateInteractionResponseMessage<'static> {
    fn from(de: CreateInteractionResponseMessageDe) -> Self {
        let mut message = CreateInteractionResponseMessage::new();
        if let Some(tts) = de.tts {
            message = message.tts(tts);
        }
        if let Some(content) = de.content {
            message = message.content(content);
        }
        if let Some(embeds) = de.embeds {
            message = message.embeds(
                embeds
                    .into_iter()
                    .map(serenity::builder::CreateEmbed::from)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(allowed_mentions) = de.allowed_mentions {
            message = message.allowed_mentions(allowed_mentions.into());
        }
        if let Some(flags) = de.flags {
            message = message.flags(flags);
        }
        if let Some(components) = de.components {
            message = message.components(
                components
                    .into_iter()
                    .map(serenity::builder::CreateComponent::from)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(poll) = de.poll {
            message = message.poll(poll.into());
        }
        message
    }
}

fn parse_response_message(
    value: &Value,
) -> Result<CreateInteractionResponseMessage<'static>, String> {
    let de: CreateInteractionResponseMessageDe = mirror(value)?;
    Ok(de.into())
}

/// Mirror of [`CreateAutocompleteResponse`].
#[derive(Debug, Deserialize)]
pub struct CreateAutocompleteResponseDe {
    #[serde(default)]
    choices: Vec<AutocompleteChoiceDe>,
}

impl From<CreateAutocompleteResponseDe> for CreateAutocompleteResponse<'static> {
    fn from(de: CreateAutocompleteResponseDe) -> Self {
        CreateAutocompleteResponse::new().set_choices(
            de.choices
                .into_iter()
                .map(AutocompleteChoice::from)
                .collect::<Vec<_>>(),
        )
    }
}

fn parse_autocomplete(value: &Value) -> Result<CreateAutocompleteResponse<'static>, String> {
    let de: CreateAutocompleteResponseDe = mirror(value)?;
    Ok(de.into())
}

/// Mirror of [`AutocompleteChoice`].
#[derive(Debug, Deserialize)]
pub struct AutocompleteChoiceDe {
    name: Cow<'static, str>,
    #[serde(default)]
    name_localizations: Option<HashMap<Cow<'static, str>, Cow<'static, str>>>,
    value: AutocompleteValueDe,
}

impl From<AutocompleteChoiceDe> for AutocompleteChoice<'static> {
    fn from(de: AutocompleteChoiceDe) -> Self {
        let mut choice = AutocompleteChoice::new(de.name, de.value.0);
        if let Some(localizations) = de.name_localizations {
            for (locale, name) in localizations {
                choice = choice.add_localized_name(locale, name);
            }
        }
        choice
    }
}

/// Mirror of the upstream-untagged [`AutocompleteValue`] enum.
///
/// Dispatch is by JSON kind: strings map to the string variant, integers to
/// `Integer(u64)`, and every other number to `Float(f64)`. A negative or
/// fractional-looking integer input therefore rebuilds through the float
/// variant, which is the closest form the builder can represent.
#[derive(Debug)]
pub struct AutocompleteValueDe(pub AutocompleteValue<'static>);

impl From<AutocompleteValueDe> for AutocompleteValue<'static> {
    fn from(de: AutocompleteValueDe) -> Self {
        de.0
    }
}

impl<'de> Deserialize<'de> for AutocompleteValueDe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let inner = match value {
            Value::String(string) => AutocompleteValue::String(string.into()),
            Value::Number(ref number) if number.as_u64().is_some() => {
                AutocompleteValue::Integer(number.as_u64().expect("checked above"))
            }
            Value::Number(number) => {
                AutocompleteValue::Float(number.as_f64().expect("JSON numbers convert to f64"))
            }
            other => {
                return Err(D::Error::custom(format!(
                    "autocomplete values are string, integer, or float, got {other}"
                )));
            }
        };
        Ok(AutocompleteValueDe(inner))
    }
}

/// Mirror of [`CreateInteractionResponseFollowup`].
///
/// Unlike the response message, upstream serializes `embeds`
/// unconditionally: an unset list rebuilds as explicit `null`. The
/// `attachments` array is accepted but dropped, like everywhere else.
#[derive(Debug, Deserialize)]
pub struct CreateInteractionResponseFollowupDe {
    #[serde(default)]
    content: Option<Cow<'static, str>>,
    #[serde(default)]
    tts: Option<bool>,
    embeds: Option<Vec<CreateEmbedDe<'static>>>,
    allowed_mentions: Option<CreateAllowedMentionsDe>,
    #[serde(default)]
    components: Option<Vec<CreateComponentDe>>,
    flags: Option<serenity::model::channel::MessageFlags>,
    poll: Option<CreatePollDe>,
    #[serde(rename = "attachments", default)]
    _attachments: Vec<Value>,
}

impl From<CreateInteractionResponseFollowupDe> for CreateInteractionResponseFollowup<'static> {
    fn from(de: CreateInteractionResponseFollowupDe) -> Self {
        let mut followup = CreateInteractionResponseFollowup::new();
        if let Some(content) = de.content {
            followup = followup.content(content);
        }
        if let Some(tts) = de.tts {
            followup = followup.tts(tts);
        }
        if let Some(embeds) = de.embeds {
            followup = followup.embeds(
                embeds
                    .into_iter()
                    .map(serenity::builder::CreateEmbed::from)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(allowed_mentions) = de.allowed_mentions {
            followup = followup.allowed_mentions(allowed_mentions.into());
        }
        if let Some(components) = de.components {
            followup = followup.components(
                components
                    .into_iter()
                    .map(serenity::builder::CreateComponent::from)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(flags) = de.flags {
            followup = followup.flags(flags);
        }
        if let Some(poll) = de.poll {
            followup = followup.poll(poll.into());
        }
        followup
    }
}
