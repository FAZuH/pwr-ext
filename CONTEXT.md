# pwr-ext

`serde::Deserialize` support for serenity-next builder types. Upstream builders are Serialize-only; this crate provides wrapper types that accept the same JSON shapes and hand back real builders.

## Language

**Wrapper**:
A pwr-ext type that mirrors a serenity-next builder's JSON shape, implements `Deserialize`, and converts into the upstream builder through its own setters. The crate's only public surface pattern.
_Avoid_: adapter, DTO, proxy

**Public surface**:
The written exposure rule: the public surface is exactly the Wrapper types, their `From<Wrapper> for Builder` impls, the prelude re-exports, and documented inherent methods a dependee consumes (`CreateMessageDe::into_canonical_value`, currently consumed by pwr-viewgen). Everything else — parse functions, mirror structs, helpers, wrapper fields, inner builders — stays crate-private even when `pub` would compile. New items join the surface only by adding a Wrapper plus its prelude entry.

One sanctioned exception: the `view_support` module, its runtime `check_*`
helpers, and `ChildRuleError` are `pub`. The `view!` macro expands in the
dependee's crate, so the helpers and error type it references must be
reachable there. The exception is named here.
_Avoid_: leaking internals, accidental pub

**Flat builder**:
A builder whose JSON form is a plain struct of fields — handled by a derived mirror struct plus setter conversion.
_Avoid_: simple builder

**Tagged tree**:
A recursive component structure whose variants are chosen by an explicit numeric `"type"` field — dispatched by hand over `serde_json::Value`.
_Avoid_: enum payload, polymorphic component

**Untagged resolution**:
Forbidden technique: serde untagged enums. Upstream's permissive fallback variants (e.g. unknown numeric kinds) make untagged silently pick the wrong branch instead of failing.
_Avoid_ entirely — never reach for it.

**Round trip**:
The crate's core test idiom: build with the real upstream builder, serialize, deserialize through the wrapper, rebuild, compare. Every covered type has one.
_Avoid_: parity check, mirror test

**Excluded type**:
A builder permanently outside the crate's scope because it has no JSON form (multipart upload bodies travel as MIME parts) or no upstream `Serialize` impl to mirror. Listed in the README, never half-supported.
_Avoid_: unsupported type, TODO

**View macro**:
The `view!` proc-macro DSL that authors serenity message views. Expands to builder construction code referencing only `::pwr_ext::view_support` paths.

**Family**:
A statically-separated message kind: legacy (`content`, `embeds`) or components v2 (`components_v2` with `IS_COMPONENTS_V2`). Mixing families is a compile error.

**Element**:
A named block in the DSL (`embed`, `action_row`, `container`, …) that maps to a builder type. Names are snake_case minus `Create`.

**Attribute**:
A `key: value` pair inside an element or at the root that maps to a builder setter. Values are arbitrary Rust expressions.

**Child rule**:
Which elements may appear under a parent, and with what cardinality (e.g. action_row ≤5 buttons or 1 select, section 1–3 text displays + 1 accessory). Enforced at compile time when children are literals, and at runtime by the emitted `check_*` helper when a splice is present.

**Registry**:
The hand-maintained table in `pwr-ext-macros` mapping element → builder type, attribute → setter, and allowed children. The macro's single source of truth for name checking and did-you-mean.

**Wrap rule**:
Parent-decided enum wrapping for children that exist in multiple trees (e.g. `action_row` under `view!` becomes `CreateComponent::ActionRow`, under `container` becomes `CreateContainerComponent::ActionRow`).

**Splice**:
A runtime-built sequence of components injected into a `view!` child position, written `{ expr }` where `expr` is an iterator of the parent's exact wrapped child type. Positionally significant: the spliced children land where authored. An `Option` spliced in acts as a conditional — `Some` contributes one item, `None` contributes none.
_Avoid_: runtime children, dynamic children

**Standalone form (`component!`)**:
The `component!` proc-macro that emits one bare builder for a single element, without the `CreateMessage` wrapper. The consumer wraps it in the parent's component enum to embed it.
_Avoid_: standalone component macro, bare macro

**Runtime law check**:
A fallible `check_*` helper in `view_support` that enforces a parent's child rule over runtime-assembled children, returning `Err(ChildRuleError)` carrying the violated law text. Emitted in generated code only when a splice is present — the spliced view itself then returns `Result`; also callable directly by a consumer doing pure runtime assembly.
_Avoid_: runtime validation, panic guard
