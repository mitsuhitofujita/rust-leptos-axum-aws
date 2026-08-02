# DR-0002: The project builds on the Leptos 0.8 line, not the 0.9 prerelease

Status: accepted
Date: 2026-08-02

## Context

At the time the frontend was set up, `cargo search leptos` listed
**0.9.0-beta** first, so the most visible version was a prerelease. The latest
stable release was **0.8.20** (`leptos_router` 0.8.15).

Leptos major-version bumps have historically carried breaking changes to the
reactive API and the router, so this choice sets the shape of the frontend code
and determines what a later upgrade will cost.

## Decision

Pin the workspace to the stable **0.8** line: `leptos` 0.8.20 with the `csr`
feature and `leptos_router` 0.8.15, declared in `[workspace.dependencies]` in
the root `Cargo.toml` so both are stated in exactly one place.

For the same reason, the build tool is stable **trunk 0.21.14** rather than
`0.22.0-beta.2`.

## Alternatives

**Adopt `leptos 0.9.0-beta` now.** It would avoid a later migration and start on
the API the project ends up on. Rejected: a beta's API can still change, its
documentation and community examples lag, and bugs found in it have no stable
fallback. Starting a long-lived project on a prerelease trades a one-time
migration cost for open-ended instability.

**Wait for 0.9 stable before starting.** Rejected: the release date is unknown
and the work is not blocked on anything 0.9 provides.

## Consequences

A migration to 0.9 will be needed eventually, and it will not be free — at
minimum `crates/app/src/app.rs` (the router and the `view!` trees) and the
`LocalResource`/`Suspense` usage in the data-fetching path will need review.

The migration is contained: it touches `crates/app` and the version numbers in
`[workspace.dependencies]`. `crates/shared` and `crates/server` do not depend on
Leptos at all, which is a property worth preserving precisely because it keeps
the blast radius of a frontend framework upgrade inside one crate.

When the move to 0.9 happens, supersede this record rather than editing it.
DR-0001 is unaffected: the CSR-plus-separate-API architecture does not depend on
which Leptos version implements it.

One detail that is easy to get wrong on 0.8: **`leptos_router` 0.8 has no `csr`
feature.** Its only features are `ssr`, `nightly`, and `tracing`, so a CSR build
depends on it with default features. Only `leptos` itself takes `features =
["csr"]`.
