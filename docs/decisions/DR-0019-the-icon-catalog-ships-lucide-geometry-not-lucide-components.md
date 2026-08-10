# DR-0019: The icon catalog ships Lucide geometry, not `lucide-leptos` components

Status: accepted
Date: 2026-08-10

## Context

DR-0014 chose Lucide for the action-type icon catalog and `lucide-leptos` as the
way to render it, and was explicit about the cost it was accepting: the crate
gates icons by category rather than individually, so "a catalog broad enough to
be worth searching compiles in whole categories at a time and is paid for in
bundle size". It named generating our own representation as the fallback "if the
crate's release cadence, its category-level feature granularity, or its bundle
cost becomes the binding constraint".

Building the catalog turned both halves of that warning into numbers.

Lucide's categories are not shaped like this application. The eight icons in the
design references alone span eight of the crate's forty-one categories —
`person-standing` is `people`, `bike` is `transportation`, `graduation-cap` is
`buildings`, `droplets` is `weather`. So category narrowing, the only lever the
crate offers, cannot produce a small catalog that is also a usable one. Fourteen
categories admit 725 icons, and that is close to the floor.

Rendering those 725 as components cost **+1.69 MB of raw wasm**, taking the
whole bundle from 721 KB to 2.41 MB. Nothing could be eliminated: the catalog is
a table the picker walks in full, so every icon is reachable by construction.

The cost was not the icons. The 725 icons are 143 KB of SVG children between
them. The remaining 1.4 MB was 725 copies of a generated Leptos component
carrying `size`, `color`, `fill`, `stroke_width`, `absolute_stroke_width` and a
derived signal — five reactive props this application never varies, on a mobile
single-page application it delivers over the network.

## Decision

`crates/icongen` reads the pinned crate's source and emits the geometry: the
children of each icon's `<svg>`, as a string, in a table beside the canonical
name and the official English name. `crates/app/src/icons.rs` writes the `<svg>`
wrapper once, with Lucide's own default attributes, and sets the geometry with
`inner_html`.

`lucide-leptos` moves off `crates/app` and onto `crates/icongen`, where no
feature of it is enabled, so its `cfg` gates exclude every icon and it compiles
to an empty library. It is depended upon rather than merely located: the
dependency is what pins the version in `Cargo.lock` and what makes cargo unpack
the source the generator reads.

Everything else DR-0014 decided stands — Lucide as the catalog, canonical
kebab-case names as the stored value, official names in the picker, both
versions pinned separately, the license notices. This revises how the geometry
reaches the browser and nothing else, in the same way DR-0014 itself revised one
detail of DR-0013.

## Alternatives

- **Accept the components.** Rejected on the measurement: 2.41 MB raw and 456 KB
  compressed, against 892 KB and 311 KB, for behaviour that is identical.
- **Narrow the catalog to a hand-picked list.** Rejected because it weakens
  DR-0013's premise. A searchable modal picker with a live result count and an
  empty state exists to make a catalog too large to scan navigable; a catalog
  small enough to make the components affordable would not need one.
- **Fetch the geometry from Lucide's own repository at generation time.**
  Rejected because it puts a network call and a second pinned version in the
  generator, when the crate already vendors the geometry at a version
  `Cargo.lock` pins.
- **Keep `lucide-leptos` on `crates/app` with no features enabled.** Rejected as
  a dependency that would be there to be unpacked rather than to be used, in the
  manifest of a crate that does not read it.

## Consequences

725 searchable icons cost +171 KB of raw wasm rather than +1.69 MB. The whole
bundle is 892 KB raw, 311 KB compressed, with the catalog in it.

The generator is now responsible for parsing a shape the crate does not promise:
one element per line between the `<svg>` opening tag and its close. That holds
in every module of 3.26.0 and it is checked — the generator fails rather than
emitting an empty icon — but it is a coupling to formatting, not to an
interface. A release that reformats its output breaks `just icons` loudly, which
is the right failure, but it does break it.

Generated markup reaches the DOM through `inner_html`, which deserves the
scrutiny its name invites. What passes through it is a string literal in a
generated file, produced at build time from a pinned crate, selected by a name
that has already matched a catalog entry exactly. No value from a user, a
request, or the store can reach it.

`cargo check --workspace` now compiles Leptos for the host, about 21 seconds
once, because the generator depends on a crate that depends on Leptos. That is
the price of the lockfile pin, and it is cached after the first build.

Upgrading the pin is now two steps rather than one: move the version, then run
`just icons`. Nothing enforces the second, so the generated files and the pin
agree only because someone ran it — which was already true of the catalog's
contents and is now true of its geometry as well.
