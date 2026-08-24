//! Wrapper types for serenity's message payload builders.

use std::borrow::Cow;

use serde::Deserialize;
use serde_json::Value;
use serenity::builder::CreateAllowedMentions;
use serenity::builder::CreateComponent;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateMessage;
use serenity::model::channel::MessageFlags;
use serenity::model::channel::MessageReference;
use serenity::model::channel::Nonce;
use serenity::model::id::RoleId;
use serenity::model::id::StickerId;
use serenity::model::id::UserId;

use crate::components::CreateComponentDe;
use crate::embed::CreateEmbedDe;

/// Mirror of [`CreateAllowedMentions`].
///
/// `parse` carries the lowercase strings upstream emits (`"everyone"`,
/// `"users"`, `"roles"`); anything else is rejected, matching upstream's own
/// private `ParseValue` enum which has no unknown-value fallback (spec D5).
/// Entries are replayed through the upstream toggle setters in payload order,
/// so duplicated entries collapse to one instead of erroring.
#[derive(Debug, Deserialize)]
pub struct CreateAllowedMentionsDe {
    #[serde(default)]
    parse: Vec<ParseValueDe>,
    #[serde(default)]
    users: Vec<UserId>,
    #[serde(default)]
    roles: Vec<RoleId>,
    replied_user: Option<bool>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ParseValueDe {
    Everyone,
    Users,
    Roles,
}

impl From<CreateAllowedMentionsDe> for CreateAllowedMentions<'static> {
    fn from(de: CreateAllowedMentionsDe) -> Self {
        let mut mentions = CreateAllowedMentions::new();
        for value in de.parse {
            mentions = match value {
                ParseValueDe::Everyone => mentions.everyone(true),
                ParseValueDe::Users => mentions.all_users(true),
                ParseValueDe::Roles => mentions.all_roles(true),
            };
        }
        mentions = mentions.users(de.users).roles(de.roles);
        if let Some(replied_user) = de.replied_user {
            mentions = mentions.replied_user(replied_user);
        }
        mentions
    }
}

/// Mirror of [`CreateMessage`].
///
/// The mirror owns its data and converts to a `CreateMessage<'static>`:
/// upstream's builder is *invariant* over its lifetime (through the
/// attachment plumbing), so a borrowing conversion cannot compile — same
/// outcome as the tag-dispatched trees, reached through variance instead of
/// dispatch.
///
/// One serialized field cannot be carried back into the builder and is
/// accepted-but-dropped: `attachments`, the JSON array shape of the
/// multipart plumbing excluded from round-trip support by spec D4 — rebuilt
/// messages always emit an empty array. `flags` deserializes through
/// upstream's `from_bits_truncate`, so bits undefined on [`MessageFlags`]
/// normalize away exactly as they do elsewhere in serenity.
///
/// `tts` and `enforce_nonce` are always serialized upstream, so they stay
/// required here.
#[derive(Debug, Deserialize)]
pub struct CreateMessageDe {
    pub content: Option<Cow<'static, str>>,
    pub nonce: Option<Nonce>,
    pub tts: bool,
    #[serde(default)]
    pub embeds: Vec<CreateEmbedDe<'static>>,
    pub allowed_mentions: Option<CreateAllowedMentionsDe>,
    pub message_reference: Option<MessageReference>,
    pub components: Option<Vec<CreateComponentDe>>,
    #[serde(default)]
    pub sticker_ids: Vec<StickerId>,
    pub flags: Option<MessageFlags>,
    #[serde(rename = "attachments", default)]
    _attachments: Vec<Value>,
    pub enforce_nonce: bool,
    #[serde(default)]
    poll: Option<crate::poll::CreatePollDe>,
}

impl From<CreateMessageDe> for CreateMessage<'static> {
    fn from(de: CreateMessageDe) -> Self {
        let mut message = CreateMessage::new();
        if let Some(content) = de.content {
            message = message.content(content);
        }
        if let Some(nonce) = de.nonce {
            message = message.nonce(nonce);
        }
        message = message.tts(de.tts);
        if !de.embeds.is_empty() {
            message = message.embeds(
                de.embeds
                    .into_iter()
                    .map(CreateEmbed::from)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(allowed_mentions) = de.allowed_mentions {
            message = message.allowed_mentions(allowed_mentions.into());
        }
        if let Some(reference) = de.message_reference {
            message = message.reference_message(reference);
        }
        if let Some(components) = de.components {
            message = message.components(
                components
                    .into_iter()
                    .map(CreateComponent::from)
                    .collect::<Vec<_>>(),
            );
        }
        if !de.sticker_ids.is_empty() {
            message = message.sticker_ids(de.sticker_ids);
        }
        if let Some(flags) = de.flags {
            message = message.flags(flags);
        }
        if let Some(poll) = de.poll {
            message = message.poll(poll.into());
        }
        message.enforce_nonce(de.enforce_nonce)
    }
}

impl CreateMessageDe {
    /// Converts into a builder and serializes it back to the canonical JSON
    /// serenity itself would emit, so downstream consumers can hand off
    /// normalized payloads without depending on serenity.
    pub fn into_canonical_value(self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(CreateMessage::from(self))
    }
}
