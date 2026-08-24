# pwr-ext

`serde::Deserialize` support for serenity-next builder types. Upstream builders are Serialize-only; this crate provides wrapper types that accept the same JSON shapes and hand back real builders.

## Language

**Wrapper**:
A pwr-ext type that mirrors a serenity-next builder's JSON shape, implements `Deserialize`, and converts into the upstream builder through its own setters. The crate's only public surface pattern.
_Avoid_: adapter, DTO, proxy

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
