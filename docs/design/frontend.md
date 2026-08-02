# Frontend

Updated: 2026-08-02

## Purpose

The user interface: a Leptos single-page application, rendered entirely in the
browser, that fetches its data from the axum API over HTTP. It is delivered as
static files and runs no server-side code of its own (DR-0001).

## Structure

The application lives in `crates/app` and is compiled to
`wasm32-unknown-unknown`. It is a binary crate — trunk builds `src/main.rs`.

| File | Role |
| --- | --- |
| `crates/app/src/main.rs` | Installs the panic hook, mounts `App` to `<body>` |
| `crates/app/src/app.rs` | The router and every component |
| `crates/app/src/api.rs` | Calls to the API, returning `shared` types |

Trunk's inputs sit at the repository root rather than inside the crate, because
the workspace manifest is virtual and cannot itself be a trunk target:

| File | Role |
| --- | --- |
| `index.html` | trunk entry point; points `data-trunk rel="rust"` at `crates/app/Cargo.toml` |
| `Trunk.toml` | dev server address and port, and the `/api` proxy |
| `style/main.css` | plain CSS, linked by `index.html` |
| `public/` | assets copied verbatim into `dist/public/` |

`trunk build` emits `dist/`: hashed `.wasm` and `.js`, hashed CSS, the copied
`public/` directory, and an `index.html` rewritten to reference them. `dist/` is
the deployable artefact and is not committed.

**Routing.** `leptos_router`'s `<Router>` wraps a `<Routes>` block declaring
`/` (`HomePage`) and `/about` (`AboutPage`), with a `NotFound` fallback.
Navigation uses `<A>`, which renders `aria-current="page"` on the active link —
the CSS styles that attribute rather than tracking the active route by hand.

**Data fetching.** `crates/app/src/api.rs` holds one async function per
endpoint. Each returns `Result<T, String>` where `T` comes from `crates/shared`,
converting a transport failure, a non-2xx status, and a decode failure into
messages the UI can render. Components load them with `LocalResource` inside
`<Suspense>`; the error branch is rendered, never swallowed.

`LocalResource`, not `Resource`: the browser fetch future is not `Send`, and in
a CSR build there is no server to run it on.

## Interfaces

**Consumes** `GET /api/greeting` → `shared::Greeting`, via a relative path. The
frontend never contains an API hostname; the origin is resolved by the proxy in
development and will be by configuration in production.

**Depends on** `leptos` (feature `csr`), `leptos_router` (default features — it
has no `csr` feature), `gloo-net` (features `http`, `json`) for fetch,
`console_error_panic_hook`, and `shared`.

**Exposes** nothing to other crates.

## Constraints

- All API calls use relative paths, so that the development proxy and the
  production origin are both handled outside the code.
- The dev server proxies `/api` to `127.0.0.1:3000`, so development is
  single-origin and CORS never arises. Production is cross-origin and requires
  CORS on the API — DR-0001.
- Deep links are router paths, not files. `trunk serve` serves `index.html` for
  unknown paths, and the production host must be configured to do the same, or
  reloading on any non-root route fails — DR-0001.
- CSS is plain and hand-written. No framework, no Node.js, no npm anywhere in
  the build.
- The framework stays inside this crate; `shared` and `server` never import
  Leptos — DR-0002.
