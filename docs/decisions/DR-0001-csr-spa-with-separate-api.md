# DR-0001: The frontend is a client-side rendered SPA served as static files, and the API is a separate service

Status: accepted
Date: 2026-08-02

## Context

The project is an axum + Leptos web application intended to run on AWS. Leptos
supports two fundamentally different shapes, and the choice had to be made
before any code existed, because it determines the build tool, the crate layout,
how types are shared, and the deployment substrate.

The two shapes are:

- **SSR full-stack.** One binary serves both rendered HTML and the API. Built by
  `cargo-leptos`, wired to axum through `leptos_axum`, with `#[server]`
  functions crossing the boundary.
- **CSR SPA plus a separate API.** A WASM bundle built by `trunk`, delivered as
  static files, calling an HTTP API over REST/JSON.

The deciding force was the intended AWS deployment. Static delivery through
S3 + CloudFront and an API on ECS or Lambda scale, deploy, and fail
independently; an SSR binary couples them into one deployable unit.

## Decision

Build a **CSR single-page application** in `crates/app`, compiled to
`wasm32-unknown-unknown` by trunk and shipped as static files, calling a
**separate axum API** in `crates/server` over REST/JSON. Types crossing the
boundary live in `crates/shared` as plain serde structs.

Consequently:

- **trunk** is the frontend build tool. `cargo-leptos` targets SSR full-stack
  builds and is not used.
- **`leptos_axum` is not a dependency.** It exists to wire Leptos SSR into axum
  and has no role in a CSR build.
- **`#[server]` functions are not used.** The contract between frontend and
  backend is an ordinary HTTP API, defined by the types in `crates/shared`.

## Alternatives

**SSR full-stack via `cargo-leptos`.** Rejected, with its advantages
acknowledged rather than dismissed: it gives real SEO and a fast first paint,
and `#[server]` functions remove the hand-written fetch layer entirely. It was
rejected because it produces a single deployable that must serve HTML, so the
frontend cannot go on a CDN as static files, and because it binds the frontend's
release cycle to the backend's. The application is not
search-engine-facing enough for the SEO advantage to outweigh that.

**Hydration (SSR plus a client-side takeover).** Carries the SSR deployment
shape as well, so it was ruled out for the same reason.

## Consequences

Easy: the frontend deploys as static objects with no runtime; frontend and API
release independently; the API is a plain HTTP service usable by any future
client; type safety across the boundary survives through `crates/shared`.

Hard, and accepted deliberately:

- **No SEO and a slower first paint.** The browser downloads a multi-megabyte
  WASM bundle before anything renders. Reversing this means re-architecting the
  build and the deployment substrate together, so the cost is high.
- **The fetch layer is written by hand.** Every endpoint needs a function in
  `crates/app/src/api.rs` and a route in `crates/server`. Nothing checks that the
  two agree beyond both using the `shared` types.

Two non-obvious constraints follow from the split, both invisible in
development and both capable of breaking production:

- **CORS.** In development trunk proxies `/api` to the API server, so the
  browser sees a single origin and CORS never arises. In production the SPA and
  the API are on different origins and the API must send CORS headers. The
  absence of a CORS layer in `crates/server` is therefore a gap that the
  deployment work has to close, not a decision that it is unnecessary.
- **SPA fallback.** Deep links such as `/about` are routes in the WASM router,
  not files. `trunk serve` falls back to `index.html` for unknown paths, which
  is why a reload works in development. The production host must be configured
  to do the same — for CloudFront, a custom error response mapping 403/404 to
  `/index.html` with status 200. Without it, every reload on a non-root route
  returns an error.

One further pitfall worth recording: **trunk and wasm-bindgen versions are
coupled.** trunk reads the `wasm-bindgen` version resolved in `Cargo.lock` and
downloads a matching `wasm-bindgen-cli` into its own cache, so the CLI must not
be installed separately or pinned by hand — doing so is how the classic
"wasm-bindgen version mismatch" failure is produced, not how it is avoided.
