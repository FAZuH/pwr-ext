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

**Emission target**: **B** — builders directly. Generated code calls public setters only (`::pwr_ext::view_support::CreateMessage::new().content(...)`). This uses `view_support` re-exports so call sites need no direct `serenity` dep. For literal children, runtime validation is not duplicated in the macro; Discord limits stay in `pwr-viewgen`'s send gate. Compile-time checks are name/type-level only (unknown element/attribute, child rule counts when literal, button style laws when literal). Runtime splices add their own checks; see the grammar-extension Decision below.

**Family separation**: explicit syntax, not inference. Legacy is the implicit root (`content`, `embed`, `action_row`, `poll`, `allowed_mentions`). Components v2 is an explicit `components_v2 { ... }` root element containing only v2 children. The macro rejects any mix in either direction at compile time and auto-ORs `IS_COMPONENTS_V2` into any user `flags` inside `components_v2`, so the flag cannot be forgotten.

## Consequences

* No public constructors are added to `De` types; the parse half stays untouched and additive.
* Callers get the full builder type system (distinct `CreateComponent` vs `CreateContainerComponent` etc.) and span-accurate diagnostics. Non-literal style/values correctly degrade to runtime checks.
* `components_v2` is the only v2 entry point; no `view_v2!` second macro is needed. One import, one family decision, statically enforced.
* Future families (modal, interaction responses) will follow the same explicit-root pattern; the registry stays extensible without changing the emission strategy.

## Decision — grammar extension: runtime splicing, `component!`

The `view!` grammar grows a runtime-splice item: a bare-brace `{ expr }` in a
child position. The expression is an iterator of the parent's **exact** wrapped
child type; no contextual Into-traits are added. The consumer wraps a bare
builder in the parent's component enum explicitly. A splice is positionally
significant — children land exactly where authored, so the pwr-bot tests that
pin the nav-row position keep passing. One splice is one iterator expression;
several runtime groups are several `{ }` items at their positions. Because
`Option<T>: IntoIterator<Item = T>`, an `Option` spliced into a position acts
as a conditional: `Some` contributes its single item, `None` contributes none.
The grammar adds no `if`/`for` child items; it stays expression-based.

Splices are accepted in every parent-child position **except** the `select_menu`
options arm, which stays literal-only. That arm's exactly-one/or-N rule is not
expressible as an `IntoIterator` splice; a runtime builder uses the public
`check_select_menu_options` helper directly.

**Runtime-law policy (a)+(b).** For a spliced parent that has a child rule,
the macro emits an inline panicking check over the combined literal+spliced
child list before the build. The check is a named public `check_*` helper in
`view_support`, one per child-rule parent kind: `check_v2_root_children`,
`check_action_row_children`, `check_container_children`,
`check_section_children`, `check_media_gallery_items`. A pure-literal parent
emits no runtime check; its compile-time checks are unchanged. The consumer
gate stays the backstop; the inline check is the first line.

`check_select_menu_options` is the same kind of helper for the literal-only
select_menu options arm. The macro does not emit it; a consumer doing runtime
options assembly calls it directly. A spliced parent with no child-rule count
law (embed fields) takes no check.

The checks are load-bearing because the pwr-bot gate validates shape through
`CreateMessageDe` only and does not enforce Discord count-laws (for example no
more than 5 buttons per row, no more than 40 components). A runtime-spliced row
could otherwise exceed a law silently. The panic names the violated law, so the
consumer sees why it failed. This matches the documented `# Panics` precedent.

The helpers are `pub` so a consumer doing pure runtime assembly can call them
directly. This is the sanctioned exception to the crate's public-surface rule.

**`component!` standalone emission.** A second proc-macro, `component!`,
accepts any single element body `view!` accepts as a child and emits the bare
builder, with no `CreateMessage` wrapper. It takes no legacy-root attributes
and no `components_v2` root. The consumer controls how the bare builder is
embedded by wrapping it in the parent's component enum variant.

**Do-not-adopt boundary vs dioxus.** The grammar borrows parser-side technique
only (raw-expression lookahead, node arm ordering) from the dioxus-rsx
reference. It does not adopt dioxus's `if`/`for` grammar (conditionals ride the
splice), its runtime VNode/template expansion, or its error-machinery scale.
The output stays typed builders, not a VNode tree.

## Consequences — grammar extension

* A spliced parent builds an owned `Vec` of children positionally (literals in
  author order, plus `.extend(splice)` at each splice site), runs the runtime
  check over the combined slice, then passes the `Vec` to `CreateX::new`. A
  pure-literal parent keeps its one-shot emission.
* The checks emit only when a splice is present, so existing trybuild output
  for literal-only calls stays byte-identical.
* `component!` and splices compose: a standalone element can itself contain
  splices.
* `view_support` now hosts both the serenity re-exports and the runtime check
  helpers. It is an explicit exception to the public-surface rule: the helpers
  are `pub` because the macro expands in the dependee's crate.
* A spliced parent gives up fully compile-time child-rule enforcement. The law
  becomes a panic at runtime, with the law in the message.

## Amendment (2026-09-06): runtime checks return `Result`, not panic

Supersedes the panic policy above (the "Runtime-law policy (a)+(b)" paragraph
and the "law becomes a panic" consequence) for the runtime `check_*` helpers.
Everything else in this ADR stands: emission target B, family separation, the
splice grammar, `component!`, and the helper-per-parent-kind layout.

**What changed.** The six `check_*` helpers in `view_support` now return
`Result<(), ChildRuleError>` instead of panicking. `ChildRuleError` is a
closed `#[non_exhaustive]` enum with one variant per law, so callers can
`match` the violation instead of parsing prose. `Display` prints the law text
verbatim and the type implements `std::error::Error` — no allocation, no new
dependency. The macro's
generated code propagates it with `?` inside the spliced expansion.

**Why.** Splices exist to assemble children from runtime data, and runtime
data is user input — panicking on it is wrong. A panic is also not matchable;
a dependee could only guard with `catch_unwind`. The checks stay advisory
(Discord remains the authority, direct children only), so an error the caller
can log-and-degrade on is the right shape, and it keeps the "named law"
diagnosis the panics provided.

**Conditional return.** `view!` returns `Result<CreateMessage, ChildRuleError>`
**iff** the view contains at least one `{ expr }` splice; `component!` follows
the same rule for its element body. Literal-only views keep returning the bare
builder — zero friction on the DSL's main path, and the guard becomes
type-driven: adding a splice to a previously literal view makes call sites
fail to compile until they handle the `Result`. Uniform Result was rejected
(it taxes literal call sites that cannot fail); this was option 5 of a
5-option tradeoff analysis, user-approved.

**Accepted over-approximation.** The rule is one-directional: splice ⇒ Result.
If a splice sits in a position that carries no child-law check (embed fields,
legacy-root component splices), the view still returns `Result` even though
its error value can never occur. Detection is a recursive walk of the parsed
items (`any_splice`), which is a superset of the positions that emit a check.

## Consequences (amendment)

* `ChildRuleError` joins `view_support` under the same public-surface
  sanctioned exception (see CONTEXT.md).
* Spliced views now surface failures as `Err` at the call site instead of
  unwinding; tests match on the law variant with `assert_eq!`.
* `# Panics` sections in the helper docs became `# Errors`.
