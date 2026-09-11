# pwr-ext

**View macros and Deserialize support for serenity-next builders.**

<hr>

<div align="center">
● <a href="#installation">Installation</a> ﻿ ● <a href="#usage">Usage</a> ﻿ ● <a href="#docs">Docs</a> ﻿ ● <a href="#license">License</a>
</div>

serenity's `Create*` builders ([`serenity::builder`](https://docs.rs/serenity/latest/serenity/builder/)) are `Serialize`-only. This crate has two halves:

- **Deserialize wrappers.** Each `XxxDe` wrapper accepts the JSON shape serenity itself emits. Discord webhook payloads carry the same shape. The wrapper converts back into the real builder through `From<Wrapper> for Builder`.
- **View macros.** `view!` and `component!` build Discord message components as a declarative tree. Literal children get compile-time checks. Runtime parts splice into fixed positions.

## Installation

The crate is not published to crates.io. Add it from git:

```sh
cargo add pwr-ext --git https://github.com/FAZuH/pwr-ext
```

To use the view macros, enable the `macros` feature:

```sh
cargo add pwr-ext --git https://github.com/FAZuH/pwr-ext --features macros
```

Requirements:

- Rust 1.95 or newer. serenity-next uses edition 2024.
- The crate tracks serenity on `branch = "next"`, not a crates.io release.
- Contributing needs nightly Rust only for formatting (`rustfmt.toml` uses unstable options).

## Usage

### Deserialize round trip

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

One prelude import brings every wrapper into scope: `use pwr_ext::prelude::*;`. A wrapper takes the upstream type name plus a `De` suffix (`CreateEmbed` → `CreateEmbedDe`). [Parse Reference](docs/parse.md) explains the wrapper shapes, the canonicalization rules, and lists full coverage.

### View macros

`view!` authors a message view. The two families are statically separated: mixing `content`/`embed` with `components_v2` is a compile error, and the macro sets `IS_COMPONENTS_V2` for you:

```rust
use pwr_ext::view;
use pwr_ext::view_support::{ButtonStyle, Colour};

// Legacy family: content + embeds + action rows
let legacy = view! {
    content: "Hello"
    embed { title: "Title", colour: Colour::from(0xFF00FF), field { name: "n", value: "v", inline: true } }
    action_row {
        button { custom_id: "btn:1", label: "Click", style: ButtonStyle::Primary }
    }
};

// Components v2 family: explicit components_v2 root
let v2 = view! {
    components_v2 {
        container {
            text_display { "Hello **v2**" }
            action_row {
                button { url: "https://example.test", label: "Link" }
            }
        }
    }
};
```

Literal children are checked at compile time: names, button style laws, and child counts (`action_row` ≤5 buttons or 1 select, `select_menu` ≤25 options). Children built at runtime splice in with `{ expr }`, and `component!` emits one bare builder. A view that contains a splice then evaluates to `Result`; a literal-only view yields the bare builder. Generated code references only `::pwr_ext::view_support`, so call sites need no direct serenity dependency. [View Macros Guide](docs/view-macros.md) covers splices and runtime checks in full.

## Docs

- [View Macros Guide](docs/view-macros.md) — the `view!` and `component!` DSL: families, splices, and runtime checks
- [Parse Reference](docs/parse.md) — wrapper shapes, naming, unknown values, canonicalizations, coverage, and exclusions
- [Domain Glossary](CONTEXT.md) — the crate's terms: wrapper, tagged tree, family, splice, child rule
- [View Macros ADR](docs/adr/0002-view-macros.md) — why the macros emit builders, and the conditional-`Result` amendment
- [Two v2 Dispatches ADR](docs/adr/0001-two-v2-dispatches-are-intentional.md) — why the component and container parsers stay separate

## License

`pwr-ext` is distributed under the terms of the [MIT](https://spdx.org/licenses/MIT.html) license.
