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
pub use pwr_ext_macros::component;
#[cfg(feature = "macros")]
pub use pwr_ext_macros::view;

/// Re-exports of serenity builder and model types referenced by the `view!`
/// macro's generated code. Call sites never need a direct `serenity`
/// dependency; the macro always emits `::pwr_ext::view_support::…` paths.
///
/// The module also hosts the runtime child-rule checks: panicking
/// counterparts of the child rules `view!` enforces at compile time for
/// literal children, for child lists assembled at runtime. Each check
/// carries the violated law in its panic message. These helpers are the
/// documented exception to the crate's public-surface rule (see
/// CONTEXT.md): they are `pub` because the macro expands in the dependee's
/// crate and the generated code must reach them via
/// `::pwr_ext::view_support::*`.
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

    /// Checks a runtime-assembled action-row button list against the
    /// `action_row` child rule the `view!` macro enforces at compile time
    /// for literal children: an action row holds up to 5 buttons or exactly
    /// one select menu — never both, and never an empty row.
    ///
    /// The buttons-vs-select-menu laws are structural in
    /// [`CreateActionRow`]: `CreateActionRow::buttons` and
    /// `CreateActionRow::select_menu` are mutually exclusive, and a
    /// select-menu row holds exactly one menu. This check enforces the laws
    /// a button list can violate.
    ///
    /// # Panics
    ///
    /// Panics when the list violates a law, with the law in the message:
    ///
    /// - no button and no select menu (an empty list builds an empty row)
    /// - more than 5 buttons
    pub fn check_action_row_children(buttons: &[CreateButton<'_>]) {
        if buttons.is_empty() {
            panic!("action_row must contain at least one button or a select menu");
        }
        if buttons.len() > 5 {
            panic!("action_row cannot contain more than 5 buttons");
        }
    }

    /// Checks runtime-assembled section children against the `section` child
    /// rule the `view!` macro enforces at compile time for literal children:
    /// 1 to 3 text displays plus exactly one accessory.
    ///
    /// The accessory is a separate argument, as in [`CreateSection::new`],
    /// so the exactly-one-accessory and text-displays-before-accessory laws
    /// are structural; the text-display count is what this check enforces.
    ///
    /// # Panics
    ///
    /// Panics when the children violate a law, with the law in the message:
    ///
    /// - no text display
    /// - more than 3 text displays
    /// - no accessory
    pub fn check_section_children(
        text_displays: &[CreateSectionComponent<'_>],
        accessory: Option<&CreateSectionAccessory<'_>>,
    ) {
        if text_displays.is_empty() {
            panic!("section must contain at least one `text_display`");
        }
        if text_displays.len() > 3 {
            panic!("section cannot contain more than 3 text_display components");
        }
        if accessory.is_none() {
            panic!("section requires exactly one accessory (`thumbnail` or `button`)");
        }
    }

    /// Checks runtime-assembled string-select options against the
    /// select-menu law the `view!` macro enforces at compile time for
    /// literal children: at most 25 options. The macro enforces no minimum,
    /// so this check does not either.
    ///
    /// # Panics
    ///
    /// Panics when the list holds more than 25 options, with the law in the
    /// message.
    pub fn check_select_menu_options(options: &[CreateSelectMenuOption<'_>]) {
        if options.len() > 25 {
            panic!("select menu cannot have more than 25 options");
        }
    }

    /// Checks runtime-assembled media-gallery items against the
    /// `media_gallery` child rule the `view!` macro enforces at compile time
    /// for literal children: 1 to 10 items.
    ///
    /// # Panics
    ///
    /// Panics when the list violates a law, with the law in the message:
    ///
    /// - no items
    /// - more than 10 items
    pub fn check_media_gallery_items(items: &[CreateMediaGalleryItem<'_>]) {
        if items.is_empty() {
            panic!("media_gallery must contain at least one `media_gallery_item`");
        }
        if items.len() > 10 {
            panic!("media_gallery cannot contain more than 10 items");
        }
    }

    /// Checks runtime-assembled container children against the container
    /// child rule the `view!` macro enforces at compile time for literal
    /// children.
    ///
    /// The rule's kind laws are structural in [`CreateContainerComponent`]:
    /// the enum has no `Container` variant, so a nested container cannot be
    /// expressed, and every variant is a legal container child. An empty
    /// child list is legal, mirroring the macro. The count law is what this
    /// check enforces: Discord caps a components-v2 message at 40 components
    /// in total, which no parent with more than 40 direct children can
    /// satisfy.
    ///
    /// # Panics
    ///
    /// Panics when the list holds more than 40 children, with the law in the
    /// message.
    pub fn check_container_children(children: &[CreateContainerComponent<'_>]) {
        if children.len() > 40 {
            panic!("container cannot contain more than 40 components");
        }
    }

    /// Checks runtime-assembled `components_v2` root children against the
    /// root child rule the `view!` macro enforces at compile time for
    /// literal children: at least one component, v2 child kinds only, and at
    /// most 40 direct children (Discord caps a components-v2 message at 40
    /// components in total, which no root with more than 40 direct children
    /// can satisfy).
    ///
    /// Every [`CreateComponent`] variant except `Label` is a legal v2 root
    /// child, so the kind law rejects modal-only labels.
    ///
    /// # Panics
    ///
    /// Panics when the children violate a law, with the law in the message:
    ///
    /// - a modal-only `label` child
    /// - no children at all
    /// - more than 40 children
    pub fn check_v2_root_children(children: &[CreateComponent<'_>]) {
        if children
            .iter()
            .any(|child| matches!(child, CreateComponent::Label(_)))
        {
            panic!("`label` cannot appear inside `components_v2` — labels are modal-only");
        }
        if children.is_empty() {
            panic!("`components_v2` must contain at least one component");
        }
        if children.len() > 40 {
            panic!("components_v2 cannot contain more than 40 components");
        }
    }
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
