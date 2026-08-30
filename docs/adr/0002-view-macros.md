# View macros emit builders, not JSON — and legacy/v2 are separate families

## Status

Accepted

## Context

The `view!` macro could be implemented in two ways:

* **A**: deserialize JSON or build wrapper types, then parse through `pwr-ext`'s `De` wrappers (`serde_json::Value` → `CreateMessageDe` → `CreateMessage`). This would reuse strict validation and give rich error messages at runtime.
* **B**: emit serenity builder construction code directly (`CreateMessage::new().content(...).embeds(...)`).

Wrapper fields in `pwr-ext` are private (deserializable via `serde`, not constructible at call sites). A proc-macro at the call site cannot build `De` structs structurally. Options to make **A** work are:

* expose public constructors on every `De` type (duplicates serenity's builder API, large surface),
* have the macro emit JSON literals and parse at runtime (`serde_json::json!` → `from_value`), paying runtime cost and losing static checking.

Discord's components v2 flag `IS_COMPONENTS_V2 (1 << 15)` makes `content`/`embeds`/`poll`/`sticker` unusable and is irrevocable per message. The DSL must prevent mixing.

## Decision

**Emission target**: **B** — builders directly. Generated code calls public setters only (`::pwr_ext::view_support::CreateMessage::new().content(...)`). This uses `view_support` re-exports so call sites need no direct `serenity` dep. Runtime validation is not duplicated in the macro; Discord limits stay in `pwr-viewgen`'s send gate. Compile-time checks are name/type-level only (unknown element/attribute, child rule counts when literal, button style laws when literal).

**Family separation**: explicit syntax, not inference. Legacy is the implicit root (`content`, `embed`, `action_row`, `poll`, `allowed_mentions`). Components v2 is an explicit `components_v2 { ... }` root element containing only v2 children. The macro rejects any mix in either direction at compile time and auto-ORs `IS_COMPONENTS_V2` into any user `flags` inside `components_v2`, so the flag cannot be forgotten.

## Consequences

* No public constructors are added to `De` types; the parse half stays untouched and additive.
* Callers get the full builder type system (distinct `CreateComponent` vs `CreateContainerComponent` etc.) and span-accurate diagnostics. Non-literal style/values correctly degrade to runtime checks.
* `components_v2` is the only v2 entry point; no `view_v2!` second macro is needed. One import, one family decision, statically enforced.
* Future families (modal, interaction responses) will follow the same explicit-root pattern; the registry stays extensible without changing the emission strategy.
