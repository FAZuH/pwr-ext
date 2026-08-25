# The two v2 component dispatches are intentional

## Status

Accepted

## Context

`CreateComponent` and `CreateContainerComponent` are the two tree-shaped
component builders serenity-next exposes for components v2. On the wire their
nodes share similar numeric `"type"` tags (action row, section, text display,
media gallery, file, separator), so pwr-ext parses them with two parallel
dispatch functions in `src/components.rs`:

- `parse_component()` — yields `CreateComponent<'static>` nodes
- `parse_container_component()` — yields `CreateContainerComponent<'static>`
  nodes, used for the children of a container

The two functions restate the same tag-to-parser mapping with a different
constructor per arm (~9 duplicated lines per direction). An architecture
review may therefore flag them as a deepening opportunity: dispatch once into
an intermediate node representation, then convert that node into whichever
upstream type the caller needs.

## Decision

The two dispatches stay parallel. They are not an accidental duplicate but a
deliberate shape.

`Component` and `ContainerComponent` are two completely different Discord
types by design ([components reference](https://docs.discord.com/developers/components/reference)):
a container accepts a strict subset of the top-level component kinds, and the
upstream builders model them as separate enums with separate variants. They
merely happen to share wire tags.

Merging the dispatches would require an intermediate node representation plus
two conversion passes out of it. That indirection must be written, threaded
through error paths, and kept in sync with upstream — a standing cost that
exceeds the ~9 duplicated lines per direction it would replace. The
duplication is shallow (same parsers, different constructors); the proposed
merge is not.

Future architecture reviews should not re-propose merging these two
dispatches. Revisit only if upstream itself unifies the two types or if the
per-arm parsing logic (not just the constructors) starts diverging from its
twin.

## Consequences

- Adding a new v2 node kind means touching both functions; the round-trip
  test suite catches a missed arm in either direction.
- The crate keeps one parse path per upstream type, so error messages name
  the exact target type being built ("container component" vs "component").
- No intermediate node enum exists to keep synchronized with serenity's
  evolving builder internals.
