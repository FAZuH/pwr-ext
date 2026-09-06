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
| Embed children | `CreateEmbedField`, `CreateEmbedImage` | Mirror |
| Components v1 | `CreateActionRow`, `CreateButton`, `CreateSelectMenu`, `CreateSelectMenuKind`, `CreateSelectMenuOption` | Mirror or in tree |
| Components v2 | `CreateComponent`, `CreateContainer`, `CreateContainerComponent`, `CreateFile`, `CreateMediaGallery`, `CreateMediaGalleryItem`, `CreateSection`, `CreateSectionAccessory`, `CreateSectionComponent`, `CreateSeparator`, `CreateTextDisplay`, `CreateThumbnail`, `CreateUnfurledMediaItem` | Mirror or in tree |
| Message | `CreateMessage`, `CreateAllowedMentions` | Mirror |
| Modal | `CreateModal`, `CreateModalComponent`, `CreateLabel` | Mirror |
| Label children | `CreateInputText`, `CreateFileUpload`, `CreateCheckbox`, `CreateCheckboxGroup`, `CreateCheckboxGroupOption`, `CreateRadioGroup`, `CreateRadioGroupOption` | In tree |
| Interaction responses | `CreateInteractionResponse`, `CreateInteractionResponseMessage`, `CreateInteractionResponseFollowup`, `CreateAutocompleteResponse` | Mirror |
| Autocomplete children | `AutocompleteChoice`, `AutocompleteValue` | Mirror |
| Polls | `CreatePoll`, `CreatePollAnswer` | Mirror |
| Soundboard | `CreateSoundboard` | Mirror |
| Guild | `CreateChannel`, `CreateForumPost`, `CreateForumTag`, `CreateInvite`, `CreateScheduledEvent`, `CreateStageInstance`, `CreateThread`, `CreateWebhook` | Mirror |
| Commands | `CreateCommand`, `CreateCommandOption`, `CreateCommandPermission` | Mirror |
| Command choices | `CreateCommandOptionChoice` | In tree |
| Misc | `CreateGuildWelcomeChannel`, `CreateRoleColours`, `CreateTestEntitlement` | Mirror |

Count: 53 of 56 public upstream `Create*` types are supported.

## View macros (feature `macros`)

An opt-in `view!` DSL for authoring serenity message views. Enable with:

```toml
pwr-ext = { path = "../pwr-ext", features = ["macros"] }
```

```rust
use pwr_ext::view;
use pwr_ext::view_support::{ButtonStyle, Colour};

// Legacy: content + embeds + action rows
let legacy = view! {
    content: "Hello"
    embed { title: "Title", colour: Colour::from(0xFF00FF), field { name: "n", value: "v", inline: true } }
    action_row {
        button { custom_id: "btn:1", label: "Click", style: ButtonStyle::Primary }
    }
};

// Components v2: explicit `components_v2` root, macro auto-sets `IS_COMPONENTS_V2`
let v2 = view! {
    components_v2 {
        container {
            accent_color: 0xFF0000,
            text_display { "Hello **v2**" }
            section {
                text_display { "Side text" }
                thumbnail { media: "https://example.test/thumb.png" }
            }
            action_row {
                button { url: "https://example.test", label: "Link" }
            }
        }
    }
};
```

Two families are statically separated: mixing `content`/`embed` with `components_v2` is a compile error. Literal checks (button style laws, `action_row` ≤5 buttons or 1 select, `section` 1–3 text displays + 1 accessory, `select_menu` ≤25 options) fire at compile time; dynamic values degrade to runtime validation. Generated code references only `::pwr_ext::view_support`, so call sites never need a direct `serenity` dependency for builder types — `use pwr_ext::view_support::*` or `pwr_ext::prelude::*` covers imports, and `serde::Deserialize` is re-exported from the prelude for generic code.

### Runtime assembly: `component!` + splices

`view!` children are compile-time literals, so a runtime 0..N piece of a fixed
shape is authored as a standalone builder with `component!` (which emits the
bare builder for one element), wrapped explicitly in the parent's component
enum, and spliced back in at its pinned position. This is the pattern the
pwr-bot settings hub uses for its discovered-plugins nav row:

```rust
use pwr_ext::component;
use pwr_ext::view;
use pwr_ext::view_support::{ButtonStyle, CreateButton, CreateContainerComponent};

// Runtime nav row: 0..N buttons from a host lookup, Option-gated on the
// discovery being non-empty. `component!` emits the bare `CreateActionRow`
// — a `Result` here, because the body contains a splice; the consumer wraps
// it in `CreateContainerComponent::ActionRow` explicitly.
let targets: Vec<&str> = host.list_plugins().unwrap_or_default();
let nav_row: Option<CreateContainerComponent<'static>> = if targets.is_empty() {
    None
} else {
    let buttons: Vec<CreateButton<'static>> = targets
        .into_iter()
        .map(|t| CreateButton::new(format!("settings:open:{t}")).label(format!("Open {t}")))
        .collect();
    Some(CreateContainerComponent::ActionRow(
        component! {
            action_row {
                { buttons } // splice the runtime buttons into the row
            }
        }
        .expect("spliced row obeys the button law"),
    ))
};

let hub = view! {
    components_v2 {
        container {
            text_display { "-# **Settings**" }
            action_row {
                button { custom_id: "settings:config:feeds", label: "Feeds", style: ButtonStyle::Secondary }
            }
            { nav_row } // spliced at its pinned position, only when non-empty
        }
        action_row {
            button { custom_id: "settings:about", label: "🛈 About", style: ButtonStyle::Secondary }
        }
    }
}
.expect("spliced view obeys the component laws");
```

Inside a `component!` body, splices behave exactly as in `view!` (the same
`expand_<element>` functions run), and the emitted builder is the bare
serenity builder — a `Result` when the body contains splices — with no
`CreateMessage` wrapping, so the consumer controls how it is embedded via
the enum variants of its parent.

Splices are allowed in every parent-child position **except** the `select_menu`
options arm, which stays literal-only. That arm has an exactly-one/or-N rule
that is not expressible as an `IntoIterator` splice; a runtime options builder
calls the public `check_select_menu_options` helper instead. In a spliced
parent that carries a child rule, the child count is checked at runtime: the
macro emits a fallible `check_*` call over the combined literal+spliced
children, and the returned error carries the violated law in the message. A
view containing at least one splice therefore evaluates to
`Result<CreateMessage, ChildRuleError>` (`component!` follows the same rule
for its element body); a pure-literal parent keeps its compile-time checks,
emits no runtime call, and still yields the bare builder.

## Exclusions

Three types stay out on purpose:

- `CreateAttachment` and `CreateSticker` are multipart upload builders. Their payload is raw file bytes, which travel as MIME parts beside the JSON and never inside it. A deserialized form would be a byte-less shell. The related `EditAttachments` plumbing inside message builders serializes part-index metadata only: this crate accepts the array but drops it, and rebuilt messages always emit an empty array.
- `CreateBotAuthParameters` does not implement `Serialize`. Its output is an OAuth URL string from its own `build()` method, so there is no JSON to mirror.

One former scope exclusion was lifted on 2026-08-23: the interaction-response family builds plain JSON bodies, so this crate now covers `CreateInteractionResponse`, `CreateInteractionResponseMessage`, `CreateInteractionResponseFollowup`, and `CreateAutocompleteResponse`.

One nuance: `CreateModal` sits in serenity's interaction-response source file, but it builds a plain JSON body. This crate covers it too.
