//! `Deserialize` support for [serenity](https://docs.rs/serenity) builder types.
//!
//! serenity's `Create*` builders (`serenity::builder`) ship as `Serialize`-only.
//! This crate adds the missing half: wrapper types carrying `Deserialize`
//! impls plus `From<Wrapper> for Builder` conversions, so builder-shaped JSON
//! (what serenity itself emits, and what webhook payloads carry) can be turned
//! back into builders. See `README.md` for a usage example and a coverage
//! table.
//!
//! # Naming
//!
//! Every wrapper is named after its upstream type with a `De` suffix
//! (`CreateButtonDe` for `CreateButton`, ...) and converts via
//! `From<CreateButtonDe> for CreateButton`. Upstream-private child types get
//! mirrors too; some stay private when users reach them only inside their
//! parent. All public wrappers are re-exported from [`prelude`].
//!
//! # The three wrapper shapes
//!
//! - **Flat mirrors with borrowing** (`CreateEmbed`, `CreateButton`, ...):
//!   a mirror struct with a derived `Deserialize` matching the exact
//!   serialized shape, converted into the real builder through public
//!   setters. String fields stay `Cow<'a, str>`. This shape exists wherever
//!   upstream variance allows borrowing.
//! - **Lifetime-free owned mirrors** (message, modal, poll, guild, command
//!   roots): same idea, but every field is owned. The reasons differ per
//!   family and each module documents its own: `CreateMessage` is invariant
//!   over its lifetime upstream, modal components are tagged unions, and
//!   data-URI validation materializes an owned string regardless.
//! - **Tag-dispatched trees** (components v1/v2, modal components): explicit
//!   numeric `"type"` dispatch over an intermediate [`serde_json::Value`],
//!   then per-variant derived mirrors. The result is always owned
//!   (`'static`).
//!
//! Never use `#[serde(untagged)]` on component mirrors: serenity's numeric
//! enums fall back to `Unknown(u8)` for *any* number, so untagged matching
//! silently resolves to whichever variant serde tries first. Explicit tag
//! dispatch is deterministic.
//!
//! # Unknown values
//!
//! Mirrors follow serenity's own semantics per type: where the upstream model
//! exposes an `Unknown(u8)` fallback (e.g. `ButtonStyle` 5/6), any value is
//! accepted; where it does not, deserialization returns an error. No silent
//! guessing. One nuance: command permissions accept kinds 1, 2, and 3 only,
//! because no builder constructor exists for other numbers.
//!
//! # Canonicalizations
//!
//! Some payloads rebuild with small, documented changes instead of failing.
//! Fields that upstream serializes without skip attributes rebuild as
//! explicit `null`s; arrays that upstream always writes rebuild as empty
//! arrays; unknown bitflag bits drop exactly like upstream's own
//! `from_bits_truncate`. Values with no public setter normalize away (stage
//! instance channel ids, privacy levels). Each rule is pinned by a test; the
//! spec in `.scratch/2026-08-23_pwr-ext/spec.md` (decision D7) holds the full
//! list.

pub mod commands;
pub mod components;
pub mod embed;
pub mod guild;
pub mod interaction;
pub mod message;
pub mod misc;
pub mod modal;
pub mod poll;
pub(crate) mod util;

#[cfg(feature = "macros")]
pub use pwr_ext_macros::view;

/// Re-exports of serenity builder and model types referenced by the `view!`
/// macro's generated code. Call sites never need a direct `serenity`
/// dependency; the macro always emits `::pwr_ext::view_support::…` paths.
pub mod view_support {
    pub use serenity::builder::CreateActionRow;
    pub use serenity::builder::CreateAllowedMentions;
    pub use serenity::builder::CreateButton;
    pub use serenity::builder::CreateComponent;
    pub use serenity::builder::CreateContainer;
    pub use serenity::builder::CreateContainerComponent;
    pub use serenity::builder::CreateEmbed;
    pub use serenity::builder::CreateEmbedAuthor;
    pub use serenity::builder::CreateEmbedFooter;
    pub use serenity::builder::CreateFile;
    pub use serenity::builder::CreateMediaGallery;
    pub use serenity::builder::CreateMediaGalleryItem;
    pub use serenity::builder::CreateMessage;
    pub use serenity::builder::CreatePoll;
    pub use serenity::builder::CreatePollAnswer;
    pub use serenity::builder::CreateSection;
    pub use serenity::builder::CreateSectionAccessory;
    pub use serenity::builder::CreateSectionComponent;
    pub use serenity::builder::CreateSelectMenu;
    pub use serenity::builder::CreateSelectMenuKind;
    pub use serenity::builder::CreateSelectMenuOption;
    pub use serenity::builder::CreateSeparator;
    pub use serenity::builder::CreateTextDisplay;
    pub use serenity::builder::CreateThumbnail;
    pub use serenity::builder::CreateUnfurledMediaItem;
    pub use serenity::model::Colour;
    pub use serenity::model::Timestamp;
    pub use serenity::model::application::ButtonStyle;
    pub use serenity::model::application::SeparatorSpacingSize;
    pub use serenity::model::channel::MessageFlags;
    pub use serenity::model::channel::PollLayoutType;
    pub use serenity::model::channel::ReactionType;
}

/// Every wrapper type this crate provides, plus `serde::Deserialize` for
/// generic call sites (`serde_json::from_value::<CreateEmbedDe>(v)` needs
/// only the type in scope, but `fn foo<T: Deserialize>(…)` needs the trait).
pub mod prelude {
    pub use serde::Deserialize;

    pub use crate::commands::CreateCommandDe;
    pub use crate::commands::CreateCommandOptionDe;
    pub use crate::commands::CreateCommandPermissionDe;
    pub use crate::components::CreateActionRowDe;
    pub use crate::components::CreateButtonDe;
    pub use crate::components::CreateComponentDe;
    pub use crate::components::CreateSelectMenuDe;
    pub use crate::components::CreateSelectMenuOptionDe;
    pub use crate::embed::CreateEmbedAuthorDe;
    pub use crate::embed::CreateEmbedDe;
    pub use crate::embed::CreateEmbedFieldDe;
    pub use crate::embed::CreateEmbedFooterDe;
    pub use crate::embed::CreateEmbedImageDe;
    pub use crate::guild::CreateChannelDe;
    pub use crate::guild::CreateForumPostDe;
    pub use crate::guild::CreateForumTagDe;
    pub use crate::guild::CreateInviteDe;
    pub use crate::guild::CreateScheduledEventDe;
    pub use crate::guild::CreateStageInstanceDe;
    pub use crate::guild::CreateThreadDe;
    pub use crate::guild::CreateWebhookDe;
    pub use crate::interaction::AutocompleteChoiceDe;
    pub use crate::interaction::AutocompleteValueDe;
    pub use crate::interaction::CreateAutocompleteResponseDe;
    pub use crate::interaction::CreateInteractionResponseDe;
    pub use crate::interaction::CreateInteractionResponseFollowupDe;
    pub use crate::interaction::CreateInteractionResponseMessageDe;
    pub use crate::message::CreateAllowedMentionsDe;
    pub use crate::message::CreateMessageDe;
    pub use crate::misc::CreateGuildWelcomeChannelDe;
    pub use crate::misc::CreateRoleColoursDe;
    pub use crate::misc::CreateTestEntitlementDe;
    pub use crate::modal::CreateLabelDe;
    pub use crate::modal::CreateModalComponentDe;
    pub use crate::modal::CreateModalDe;
    pub use crate::poll::CreatePollAnswerDe;
    pub use crate::poll::CreatePollDe;
    pub use crate::poll::CreateSoundboardDe;
}
