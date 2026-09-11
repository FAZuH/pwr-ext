# View Macros Guide

`view!` and `component!` build Discord message components as a declarative
tree. They come from the `pwr-ext-macros` crate behind the `macros` feature.
The macros emit serenity builder construction code, not JSON. Generated code
references only `::pwr_ext::view_support`, so call sites need no direct
serenity dependency. `use pwr_ext::view_support::*` or `use
pwr_ext::prelude::*` covers the imports; `serde::Deserialize` is re-exported
from the prelude for generic code.

Enable the feature:

```toml
pwr-ext = { git = "https://github.com/FAZuH/pwr-ext", features = ["macros"] }
```

## Families

Two families are statically separated: the implicit legacy root
(`content`, `embed`, `action_row`, `poll`, `allowed_mentions`) and the
explicit `components_v2` root. Mixing `content`/`embed` with `components_v2`
is a compile error. Inside `components_v2`, the macro auto-sets
`IS_COMPONENTS_V2` and OR-s it into any user `flags`, so the flag cannot be
forgotten.

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

## Compile-time checks

For literal children, the macro checks names and types, and enforces the
child rules: button style laws, `action_row` ≤5 buttons or 1 select,
`section` 1–3 text displays + 1 accessory, `select_menu` ≤25 options.
Dynamic values degrade to runtime validation.

## Runtime assembly: `component!` + splices

`view!` children are compile-time literals, so a runtime 0..N piece of a
fixed shape is authored as a standalone builder with `component!` (which
emits the bare builder for one element), wrapped explicitly in the parent's
component enum, and spliced back in at its pinned position. This is the
pattern the pwr-bot settings hub uses for its discovered-plugins nav row:

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

Splices are allowed in every parent-child position **except** the
`select_menu` options arm, which stays literal-only. That arm has an
exactly-one/or-N rule that is not expressible as an `IntoIterator` splice; a
runtime options builder calls the public `check_select_menu_options` helper
instead. In a spliced parent that carries a child rule, the child count is
checked at runtime: the macro emits a fallible `check_*` call over the
combined literal+spliced children, and the returned error carries the
violated law in the message. A view containing at least one splice therefore
evaluates to `Result<CreateMessage, ChildRuleError>` (`component!` follows
the same rule for its element body); a pure-literal parent keeps its
compile-time checks, emits no runtime call, and still yields the bare
builder.

## Return type

The rule is one-directional: a splice makes the view return `Result`, even
where no child law applies. Adding a splice to a previously literal view
makes call sites fail to compile until they handle the `Result`. See
[ADR 0002](adr/0002-view-macros.md), in particular the 2026-09-06 amendment
that replaced the original panic policy with `Result`.

## Runtime check helpers

`view_support` exposes one fallible `check_*` helper per child-rule parent
kind. The macro emits them only when a splice is present. A consumer doing
pure runtime assembly can call them directly. Each returns
`Result<(), ChildRuleError>`; `ChildRuleError` is a `#[non_exhaustive]` enum
with one variant per law, and its `Display` prints the law text.

- `check_v2_root_children`
- `check_container_children`
- `check_action_row_children`
- `check_section_children`
- `check_media_gallery_items`
- `check_select_menu_options`

These helpers and `ChildRuleError` are the sanctioned exception to the
crate's public-surface rule (see [CONTEXT.md](../CONTEXT.md)): the macro
expands in the dependee's crate, so the generated code must reach them.
