# pwr-ext

`Deserialize` support for [serenity](https://docs.rs/serenity) builder types.

serenity's request builders in [`serenity::builder`](https://docs.rs/serenity/latest/serenity/builder/) ship as `Serialize`-only. This crate adds the missing half. Each wrapper carries a `Deserialize` impl and converts back into the real builder through `From<Wrapper> for Builder`. The target JSON shape is what serenity itself emits. That is also the shape Discord webhook payloads carry.

This crate tracks serenity on `branch = "next"` at commit `37b9f433`. It needs Rust 1.95 or newer, because serenity-next uses edition 2024. The crate has no dependency on `pwr-viewgen`; it moves to its own repository later.

## Usage

Round-trip a builder through JSON:

```rust
use pwr_ext::prelude::*;
use serenity::builder::CreateButton;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let button = CreateButton::new("verify:start").label("Start");

    // Serialize the builder. This is the half serenity already provides.
    let json = serde_json::to_value(&button)?;

    // Deserialize it back into a builder with this crate.
    let de: CreateButtonDe = serde_json::from_value(json.clone())?;
    let rebuilt: serenity::builder::CreateButton = de.into();

    // The rebuilt builder serializes to the same JSON.
    assert_eq!(serde_json::to_value(&rebuilt)?, json);
    Ok(())
}
```

Every wrapper lives behind one prelude import:

```rust
use pwr_ext::prelude::*;
```

## Naming scheme

A wrapper takes the upstream type name plus a `De` suffix. `CreateEmbed` maps to `CreateEmbedDe`, and the conversion is `From<CreateEmbedDe> for CreateEmbed`. Upstream-private child types (embed fields, poll answer media, label components) get mirrors too. Some stay private when users reach them only inside their parent.

## How the wrappers work

Three shapes cover every type in this crate.

**Flat mirrors with borrowing.** Simple builders such as `CreateEmbed` and `CreateButton` get a mirror struct with a derived `Deserialize`. The fields match the serialized shape exactly, and string fields stay `Cow<'a, str>`. Conversion calls public setters. This shape exists where upstream variance allows borrowing.

**Lifetime-free owned mirrors.** Later families (message, modal, poll, guild, commands) own their strings. Reasons differ per family, and each module documents its reason. For `CreateMessage` the cause is upstream variance: the builder is invariant over its lifetime, so a borrowing conversion cannot compile. For soundboard and webhook avatars, data-URI validation creates an owned string anyway.

**Tag-dispatched trees.** Component trees carry a numeric `"type"` tag on every node. The parser captures the node as a `serde_json::Value`, reads the tag, and then deserializes a matching mirror. The result is always owned (`'static`). Never replace this with `#[serde(untagged)]`: serenity's numeric enums fall back to `Unknown(u8)` for any number, so untagged matching silently resolves to whichever variant serde tries first. Explicit dispatch is deterministic.

## Unknown values

Mirrors copy serenity's own semantics per type. Where the upstream enum exposes an `Unknown(u8)` fallback, any number passes (examples: `ButtonStyle` 5 and 6, `ChannelType`, `PollLayoutType`). Where upstream offers no fallback, deserialization returns an error. One nuance: command permissions accept kinds 1, 2, and 3 only, because no builder constructor exists for other numbers.

## Canonicalizations

Some payloads rebuild with small, documented changes instead of failing. The rules follow upstream behavior:

- An unset embed `type` rebuilds as `"rich"`.
- Fields upstream serializes without skip attributes rebuild as explicit `null`s. Examples: embed author and footer URLs, label descriptions, option bounds, poll layout type, poll answer media, and forum tag emojis.
- Arrays upstream always writes rebuild as empty arrays. Examples: message `embeds` and `sticker_ids`, allowed-mentions `parse`, `users`, and `roles`.
- Empty arrays that upstream skips disappear. Examples: `file_types` and applied tags.
- Unknown bitflag bits drop, exactly like upstream's `from_bits_truncate`.
- Duplicate allowed-mentions `parse` entries collapse, because the toggle setters deduplicate.
- Checkbox groups replay the coupled setters in builder order. A zero `min_values` gains `"required": false`.
- Values with no public setter normalize away. Stage-instance `channel_id` rebuilds as `null`; privacy levels rebuild as GuildOnly.
- A command with null `name` rebuilds with an empty-string name, because upstream cannot go back to null.
- Permissions serialize back as strings.

Each rule has a test that pins it.

## Coverage

The table lists every upstream `Create*` type against this crate. "Mirror" means a public wrapper; "in tree" means the parent parses it internally.

| Family | Types | Support |
|---|---|---|
| Embeds | `CreateEmbed`, `CreateEmbedAuthor`, `CreateEmbedFooter` | Mirror |
| Embed children | `CreateEmbedField`, `CreateEmbedImage` | In tree |
| Components v1 | `CreateActionRow`, `CreateButton`, `CreateSelectMenu`, `CreateSelectMenuKind`, `CreateSelectMenuOption` | Mirror |
| Components v2 | `CreateComponent`, `CreateContainer`, `CreateContainerComponent`, `CreateFile`, `CreateMediaGallery`, `CreateMediaGalleryItem`, `CreateSection`, `CreateSectionAccessory`, `CreateSectionComponent`, `CreateSeparator`, `CreateTextDisplay`, `CreateThumbnail`, `CreateUnfurledMediaItem` | Mirror or in tree |
| Message | `CreateMessage`, `CreateAllowedMentions` | Mirror |
| Modal | `CreateModal`, `CreateModalComponent`, `CreateLabel` | Mirror |
| Label children | `CreateInputText`, `CreateFileUpload`, `CreateCheckbox`, `CreateCheckboxGroup`, `CreateCheckboxGroupOption`, `CreateRadioGroup`, `CreateRadioGroupOption` | In tree |
| Interaction responses | `CreateInteractionResponse`, `CreateInteractionResponseMessage`, `CreateInteractionResponseFollowup`, `CreateAutocompleteResponse` | Mirror |
| Autocomplete children | `AutocompleteChoice`, `AutocompleteValue` | In tree |
| Polls | `CreatePoll`, `CreatePollAnswer` | Mirror |
| Soundboard | `CreateSoundboard` | Mirror |
| Guild | `CreateChannel`, `CreateForumPost`, `CreateForumTag`, `CreateInvite`, `CreateScheduledEvent`, `CreateStageInstance`, `CreateThread`, `CreateWebhook` | Mirror |
| Commands | `CreateCommand`, `CreateCommandOption`, `CreateCommandPermission` | Mirror |
| Command choices | `CreateCommandOptionChoice` | In tree |
| Misc | `CreateGuildWelcomeChannel`, `CreateRoleColours`, `CreateTestEntitlement` | Mirror |

Count: 53 of 56 public upstream `Create*` types are supported.

## Exclusions

Three types stay out on purpose:

- `CreateAttachment` and `CreateSticker` are multipart upload builders. Their payload is raw file bytes, which travel as MIME parts beside the JSON and never inside it. A deserialized form would be a byte-less shell. The related `EditAttachments` plumbing inside message builders serializes part-index metadata only: this crate accepts the array but drops it, and rebuilt messages always emit an empty array.
- `CreateBotAuthParameters` does not implement `Serialize`. Its output is an OAuth URL string from its own `build()` method, so there is no JSON to mirror.

One former scope exclusion was lifted on 2026-08-23: the interaction-response family builds plain JSON bodies, so this crate now covers `CreateInteractionResponse`, `CreateInteractionResponseMessage`, `CreateInteractionResponseFollowup`, and `CreateAutocompleteResponse`.

One nuance: `CreateModal` sits in serenity's interaction-response source file, but it builds a plain JSON body. This crate covers it too.
