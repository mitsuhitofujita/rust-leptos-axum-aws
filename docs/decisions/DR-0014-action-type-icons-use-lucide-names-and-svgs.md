# DR-0014: Action type icons use Lucide names and SVGs
Status: accepted
Date: 2026-08-09

## Context

Action types need a catalog that can grow beyond the initial eight illustrative
glyphs without creating and naming every icon locally. The catalog must remain
visually coherent at 20 pixels, work in static HTML references and the Rust
frontend, and provide stable searchable names.

DR-0012 answered this with a catalog the application owns outright — its own
paths under its own identifiers. That ownership is what this record replaces.
Everything else DR-0012 settled — one stored identifier per action type, no
uploads, no raw SVG, no free-text name — is retained here and in DR-0013.

Lucide supplies a large, consistently styled set of 24-pixel SVG line icons,
canonical names, and static assets under a permissive license. The frontend does
not need a JavaScript icon runtime to render those SVGs.

## Decision

The action-type icon catalog uses Lucide, initially pinned to version 1.30.0.
An action type stores the canonical Lucide kebab-case icon name, such as
`person-standing` or `book-open`. The picker shows Lucide's corresponding
human-readable name, such as `Person Standing` or `Book Open`, and searches
those names.

The application renders pinned Lucide SVG geometry locally. Design references
inline that geometry, and the implementation should use a locally bundled or
generated Rust representation rather than a runtime CDN or a browser-side
Lucide JavaScript package. `lucide-leptos` — 3.26.0 at the time of writing,
built against the Leptos 0.8 line this workspace stays on (DR-0002) — is the
intended source of that representation.

Two version numbers therefore have to be pinned separately, because the crate's
version does not encode the upstream Lucide version it vendors. The supported
catalog is whatever the pinned crate provides; the Lucide version above names
the geometry the design references were drawn from.

The repository retains the required Lucide license notice, verbatim from
upstream, in `THIRD_PARTY_NOTICES.md` at the repository root.

This record also revises one detail of DR-0013: the collapsed form control is
visually icon-only rather than showing the icon beside its text name. The
adjacent `Icon` field label supplies its purpose, while a visually hidden
current Lucide name keeps the selected value available to assistive technology.
Official names remain visible in the expanded picker, where they support
recognition and search. Everything else DR-0013 decided about the picker stands.

## Alternatives

- **Continue the application-owned catalog.** Rejected because maintaining a
  sufficiently broad set of paths, names, and stylistic consistency would
  duplicate work already handled by a mature icon project.
- **Load Lucide from a CDN at runtime.** Rejected because icons are core UI and
  should not depend on a third-party request, changing remote assets, or
  browser-side replacement logic.
- **Store activity-specific aliases such as `running` or `water`.** Rejected
  because aliases create a second namespace that can drift from the library and
  make catalog upgrades harder to validate.
- **Store arbitrary user-entered Lucide names.** Rejected because invalid or
  removed names would create broken presentation. The searchable picker remains
  the controlled input surface.
- **Generate our own Rust modules from the Lucide SVG assets.** Not chosen for
  now, because it reintroduces the maintenance `lucide-leptos` removes. It stays
  the fallback if the crate's release cadence, its category-level feature
  granularity, or its bundle cost becomes the binding constraint.

## Consequences

The picker and every action-type display share one recognizable, expandable
icon language. Canonical names make stored values predictable and visible names
make search understandable. The application must pin both versions, preserve
license notices, validate stored names against the supported catalog, and handle
aliases or removals deliberately when upgrading the catalog.

Two costs follow from `lucide-leptos` specifically, and neither is visible from
the design references alone. Its icons are generated components, not a runtime
lookup, so the stored kebab-case name reaches its glyph only through a mapping
this project generates and keeps in step with the pinned crate — the same
generated table the picker searches. And its features gate icons by category
rather than individually, so a catalog broad enough to be worth searching
compiles in whole categories at a time and is paid for in bundle size. Narrowing
to a few categories, or generating only the supported catalog, are the levers
if that cost bites.
