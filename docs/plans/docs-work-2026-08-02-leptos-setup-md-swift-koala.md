# Leptos frontend setup — implementation plan

Executes `docs/work/2026-08-02-leptos-setup.md` (Work Log).

## Context

The repository is currently documentation only — a devcontainer, a LICENSE, a
README, and `docs/`. There is no Rust code at all. The Work Log for this unit of
work has already been agreed with the user: build an axum + Leptos web
application, and in this unit deliver **the Leptos (frontend) setup**.

The architecture is settled in the Work Log's Clarifications: a **CSR SPA built
by trunk** (eventually served as static files from S3 + CloudFront) talking to a
**separate axum API** over REST/JSON, in a **Cargo workspace of three crates**
(`app`, `server`, `shared`). This plan turns that into files.

The backend proper, AWS deployment, CSS frameworks, and a test suite are out of
scope — only the minimal axum server needed as a real fetch target is created.

Documentation is governed by `docs/README.md`: the Work Log is appended to as
work proceeds, durable knowledge is extracted into Decision Records, and Design
Documents are drafted but confirmed by the user before the work is called done.

## Decisions taken with the user (2026-08-02)

- **Tooling install**: unpack the prebuilt trunk binary into the running
  container now *and* record the same step in `.devcontainer/Dockerfile` so it
  survives a rebuild. wasm-bindgen is **not** installed explicitly — trunk
  downloads a matching `wasm-bindgen` into its own cache at build time.
- **Verification**: I run everything checkable over HTTP (build, clippy, curl
  against `trunk serve` and the API). The final browser eyeball — console clean,
  routing renders — is handed to the user with exact steps.
- **Decision Records**: two records, DR-0001 (CSR SPA + separate API) and
  DR-0002 (stay on the Leptos 0.8 line), so a future 0.9 migration can supersede
  DR-0002 alone.

## Versions (confirmed on crates.io, 2026-08-02)

leptos 0.8.20 · leptos_router 0.8.15 · axum 0.8.9 · tokio 1.53.1 ·
serde 1.0.229 · serde_json 1.0.151 · gloo-net 0.7.0 · tower-http 0.7.0 ·
console_error_panic_hook 0.1.7 · trunk 0.21.14 · wasm-bindgen 0.2.126

`leptos 0.9.0-beta` and `trunk 0.22.0-beta.2` exist but are prereleases and are
not used. `leptos_router` 0.8 has **no** `csr` feature (only `ssr`/`nightly`/
`tracing`) — depend on it with default features.

Prebuilt binary confirmed reachable (HTTP 200):
`https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-gnu.tar.gz`

## Steps

### 1. Toolchain

- Add `rust-toolchain.toml` at the root: `channel = "1.96.1"`,
  `components = ["clippy", "rustfmt"]`,
  `targets = ["x86_64-unknown-linux-gnu", "wasm32-unknown-unknown"]`.
  rustup installs the wasm target on the next cargo invocation.
- Install trunk 0.21.14 into `~/.cargo/bin` from the tarball above.
- Append the same download-and-unpack step to `.devcontainer/Dockerfile`, after
  the existing `rustup component add` line, plus
  `rustup target add wasm32-unknown-unknown`. Pin the version literally so the
  image is reproducible.
- Verify: `trunk --version`, `rustup target list --installed`.

### 2. Workspace skeleton

- Root `Cargo.toml`: virtual manifest, `members = ["crates/*"]`, `resolver = "3"`,
  and every third-party version centralised in `[workspace.dependencies]`.
  Crates reference them with `dep.workspace = true`.
- Root `.gitignore`: `/target`, `/dist`, `.trunk/`.

### 3. `crates/shared`

Plain library, `serde` with `derive` only (no wasm/tokio dependency, so it
compiles for both targets):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Greeting { pub message: String }
```

This is the single type that proves the round trip; it is used by name in both
`crates/app` and `crates/server`.

### 4. `crates/app` — the centre of this work

- `Cargo.toml`: `leptos = { workspace = true, features = ["csr"] }`,
  `leptos_router`, `console_error_panic_hook`,
  `gloo-net = { default-features = false, features = ["http", "json"] }`,
  `shared`. `crate-type` stays the default bin — trunk builds `src/main.rs` for
  `wasm32-unknown-unknown`.
- `src/main.rs`: set the panic hook, `leptos::mount::mount_to_body(App)`.
- `src/app.rs`: `<Router>` with a nav of `<A>` links and
  `<Routes fallback=…>` over two routes — `path!("/")` → `HomePage`,
  `path!("/about")` → `AboutPage` — plus a not-found fallback.
- `src/api.rs`: `pub async fn fetch_greeting() -> Result<Greeting, String>`
  using `gloo_net::http::Request::get("/api/greeting")`, returning the `shared`
  type.
- `HomePage` uses `LocalResource::new(…)` inside `<Suspense>` to render the
  fetched `Greeting`, with the error branch rendered rather than swallowed.
  (`LocalResource` is the CSR-correct choice — the future is not `Send`.)

### 5. Root-level trunk assets

The Work Log fixes `style/` and `public/` at the repository root, so trunk is
driven from the root and pointed at the app crate's manifest — a virtual
workspace manifest cannot be a trunk target.

- `index.html` (root):
  `<link data-trunk rel="rust" href="crates/app/Cargo.toml" />`,
  `<link data-trunk rel="css" href="style/main.css" />`,
  `<link data-trunk rel="copy-dir" href="public" />`.
- `Trunk.toml`: `[serve]` on port 8080 with `address = "0.0.0.0"` (so the
  devcontainer port forward reaches it), and
  `[[proxy]] backend = "http://127.0.0.1:3000/api"` so `/api/*` reaches axum on
  one apparent origin during development.
- `style/main.css`: minimal plain CSS — layout, nav, readable defaults. No
  framework.
- `public/`: a `favicon` placeholder so the directory is real and `copy-dir`
  has something to copy.

### 6. `crates/server`

Minimal axum 0.8 + tokio (`features = ["rt-multi-thread", "macros", "net"]`):

- `GET /health` → `"ok"`
- `GET /api/greeting` → `Json(Greeting { … })`, the same `shared` type
- Binds `127.0.0.1:3000`, logs the bound address.

No CORS layer: development goes through the trunk proxy, so it is same-origin.
Production runs on separate origins and *will* need CORS — that belongs to the
AWS deployment unit of work and is recorded as a pitfall in DR-0001, not coded
here.

### 7. `justfile`

Recipes: `dev-web` (`trunk serve`), `dev-api` (`cargo run -p server`), `build`
(`cargo build --workspace` + `trunk build --release`), `check`, `lint`
(`cargo clippy --workspace --all-targets -- -D warnings`), `fmt`, `fmt-check`.
Note that `.claude/settings.json` already allows `just build` / `just lint` /
`just test` and denies `just run`, so the frontend/API dev recipes are named
`dev-web` / `dev-api` and are started by the user, not silently by me.

### 8. Documentation (per `docs/README.md`)

- Append a dated entry to the Work Log's **Progress** as each step lands,
  including anything that turns out differently from this plan. Superseded plan
  steps are marked, not rewritten.
- Write `docs/decisions/DR-0001-csr-spa-with-separate-api.md` and
  `docs/decisions/DR-0002-stay-on-leptos-0-8.md` following the Decision Record
  template (Context / Decision / Alternatives / Consequences). DR-0001 carries
  the non-obvious knowledge: the SSR-full-stack alternative and why it was
  rejected, `cargo-leptos`/`leptos_axum` being SSR-only and therefore unused,
  trunk↔wasm-bindgen version coupling, and the dev-proxy vs production-CORS gap
  (plus the S3/CloudFront SPA-fallback requirement that mirrors trunk's dev
  fallback).
- Draft `docs/design/index.md`, `docs/design/frontend.md`, and
  `docs/design/workspace.md`, then **stop and have the user confirm them** —
  Design Documents are overwrite-oriented and require human sign-off.
- Tick the Retirement checklist only after that confirmation; the Work Log is
  deleted in a follow-up once the user agrees it is spent.

## Verification

Run in order; all of these are mine to execute.

1. `rustup target list --installed` shows `wasm32-unknown-unknown`;
   `trunk --version` reports 0.21.14.
2. `cargo fmt --all --check`.
3. `cargo check --workspace` and
   `cargo clippy --workspace --all-targets -- -D warnings` — clean.
4. `cargo check -p app --target wasm32-unknown-unknown` — the app crate builds
   for wasm specifically.
5. `trunk build` — `dist/` contains `index.html`, a `.wasm`, and a `.js` glue
   file; the CSS and `public/` contents are present.
6. `cargo run -p server` in the background, then
   `curl -s localhost:3000/health` → `ok` and
   `curl -s localhost:3000/api/greeting` → the expected JSON.
7. `trunk serve` in the background, then
   `curl -s localhost:8080/` returns the built HTML,
   `curl -s localhost:8080/api/greeting` returns the same JSON **through the
   proxy**, and `curl -s localhost:8080/about` returns index.html (SPA fallback,
   which is what makes reload-on-a-route work).
8. Stop both background processes.

Then hand to the user, with both dev servers running:

- open `http://localhost:8080/`, confirm the greeting from the API renders and
  the browser console is empty;
- click through to `/about` and back;
- reload while on `/about` and confirm it still renders.

Only after that does the work move to the documentation sign-off in step 8.
