# pwr-ext

`serde::Deserialize` support for serenity-next builder types. Upstream builders are Serialize-only; this crate provides wrapper types that accept the same JSON shapes and hand back real builders.

## Language

**Wrapper**:
A pwr-ext type that mirrors a serenity-next builder's JSON shape, implements `Deserialize`, and converts into the upstream builder through its own setters. The crate's only public surface pattern.
_Avoid_: adapter, DTO, proxy

**Public surface**:
The written exposure rule: the public surface is exactly the Wrapper types, their `From<Wrapper> for Builder` impls, the prelude re-exports, and documented inherent methods a dependee consumes (`CreateMessageDe::into_canonical_value`, currently consumed by pwr-viewgen). Everything else — parse functions, mirror structs, helpers, wrapper fields, inner builders — stays crate-private even when `pub` would compile. New items join the surface only by adding a Wrapper plus its prelude entry.
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
Which elements may appear under a parent, and with what cardinality (e.g. action_row ≤5 buttons or 1 select, section 1–3 text displays + 1 accessory). Enforced at compile time when children are literals.

**Registry**:
The hand-maintained table in `pwr-ext-macros` mapping element → builder type, attribute → setter, and allowed children. The macro's single source of truth for name checking and did-you-mean.

**Wrap rule**:
Parent-decided enum wrapping for children that exist in multiple trees (e.g. `action_row` under `view!` becomes `CreateComponent::ActionRow`, under `container` becomes `CreateContainerComponent::ActionRow`).
