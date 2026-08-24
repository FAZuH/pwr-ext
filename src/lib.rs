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

/// Every wrapper type this crate provides.
pub mod prelude {
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
