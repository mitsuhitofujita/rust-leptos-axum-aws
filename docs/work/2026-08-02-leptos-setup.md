# Leptos frontend setup

Status: complete
Started: 2026-08-02
Completed: 2026-08-02
Branch: main

## Request

Build a web application with axum on the backend and Leptos on the frontend.
This unit of work covers the Leptos setup only. Read `docs/README.md` first and
follow the documentation model it describes.

### Clarifications

2026-08-02 — two questions were asked and answered.

**Rendering architecture.** CSR SPA with a separate API, rather than an SSR
full-stack binary. A WASM SPA built by trunk is served as static files from S3 +
CloudFront and talks to an axum API over REST/JSON; on AWS the static delivery
and the API are deployed and managed independently.

**Crate layout.** A Cargo workspace split into three crates: `crates/app` for
the Leptos components, `crates/server` for axum and the binary entry point, and
`crates/shared` for types used by both, with `style/` and `public/` at the root.

## Interpretation

### What is being asked

An axum + Leptos web application that will eventually run on AWS. This unit of
work is the **frontend (Leptos) setup**, delivering:

- A workspace root `Cargo.toml` and the skeleton of `crates/{app,server,shared}`
- A CSR Leptos application in `crates/app` that renders in the browser under
  `trunk serve`, with working routing and dynamic rendering
- At least one type in `crates/shared` that is genuinely referenced from both
  the app and the server
- A minimal axum server in `crates/server` exposing one API that returns a
  `shared` type — enough to be a real fetch target for the frontend, not the
  backend implementation itself
- An established way to run the frontend (trunk) and the API (axum) side by side
  during development

### What is out of scope

- **The backend proper** — domain logic, database, authentication. Only the
  minimal axum server needed as a fetch target is created here.
- **AWS deployment** — S3 / CloudFront / ECS / Lambda, IaC, CI/CD. The
  architectural direction is settled, but no resource definitions and no
  deployment are produced in this unit of work.
- **CSS frameworks and design systems** (Tailwind and similar). Plain CSS, kept
  to a minimum.
- **A test suite.** Verification is by build plus manual end-to-end checks.
- **Moving to Leptos 0.9.** The 0.8 line is used, for the reason recorded below.

### Assumptions

- Leptos is pinned to the current stable **0.8** line. `0.9.0-beta` exists but is
  a prerelease and is not a base to build on for the long term. Latest stable
  versions confirmed on crates.io: leptos 0.8.20, leptos_router 0.8.15,
  axum 0.8.9, tokio 1.53.1.
- Because the build is CSR, the build tool is **trunk**. `cargo-leptos` targets
  SSR full-stack builds and is not used; `leptos_axum` is likewise SSR-only and
  is not needed.
- The devcontainer ships Rust 1.96.1 only. The `wasm32-unknown-unknown` target,
  trunk, and wasm-bindgen-cli are all absent and will be installed as part of
  this work.
- The Dockerfile already installs `just`, which is read as intent to use a
  `justfile` as the task runner.
- During development, trunk's proxy forwards `/api` to the axum server so both
  appear to be on one origin. Production will use separate origins and will need
  CORS configuration, but that belongs to the AWS deployment unit of work. Only
  the dev proxy is set up here.
- No dependency on Node.js or npm.
- The Git remote's repository name differs from the local directory name;
  aligning them is out of scope here.

## Plan

1. **Toolchain**
   Pin 1.96.1 and `wasm32-unknown-unknown` via `rust-toolchain.toml`; install
   trunk and wasm-bindgen-cli. How to install them (build time) is decided after
   trying it.

2. **Workspace skeleton**
   Root `Cargo.toml` (workspace, `members = crates/*`, dependency versions
   centralised in `[workspace.dependencies]`) and `.gitignore` (`target/`,
   `dist/`).

3. **`crates/shared`**
   Define the serde types shared by frontend and backend — one minimal type
   sufficient to prove the API round trip.

4. **`crates/app`** — the centre of this work
   leptos 0.8 with the `csr` feature, leptos_router, and
   console_error_panic_hook; a SPA with routing. Add `index.html` and
   `Trunk.toml` (proxying `/api` to axum on localhost), and one screen that
   fetches the `shared` type and renders it. Minimal plain CSS.

5. **`crates/server`**
   A minimal axum 0.8 + tokio server: a health check and one API returning the
   `shared` type. Nothing further in this unit of work.

6. **`justfile`**
   Tasks for frontend dev, API dev, build, and check/clippy/fmt.

7. **Verification**
   As set out below.

8. **Documentation**
   The choice of CSR + separate API needs a Decision Record (see Progress).
   Design Documents are drafted when the work completes and confirmed by the
   user before the work is called done.

## Progress

### 2026-08-02

Surveyed the environment. Rust 1.96.1 / cargo 1.96.1 only; the sole installed
target is `x86_64-unknown-linux-gnu`. cargo-leptos, trunk, wasm-pack, and sass
are all absent. The devcontainer is based on `rust:1.96-slim-trixie` and
includes curl, git, jq, just, ripgrep, clippy, and rustfmt.

Checked latest stable versions on crates.io: leptos 0.8.20, leptos_axum 0.8.10,
leptos_router 0.8.15, cargo-leptos 0.3.7, axum 0.8.9, tokio 1.53.1.
`cargo search` lists leptos 0.9.0-beta first, but it is a prerelease.

Confirmed the rendering architecture and crate layout with the user: CSR SPA
with a separate API, and a split workspace (see Clarifications above).

**Decisions that warrant a Decision Record (not yet written):**

- **Choosing a CSR SPA with a separate API over an SSR full-stack build.**
  A concrete alternative (a single SSR binary via cargo-leptos) was considered,
  and the trade-offs were accepted deliberately: AWS deployment shape
  (S3 + CloudFront plus ECS/Lambda versus one container), SEO, how type sharing
  is handled, and first-paint latency. Reversing it means rebuilding both the
  build setup and the deployment substrate, so the cost is high.
  → Should be raised as DR-0001.

- **Staying on the Leptos 0.8 line rather than adopting 0.9.0-beta.**
  0.9 is expected to carry breaking changes, so a migration decision will come
  up eventually, and whoever faces it will want to know why 0.8 was the starting
  point. Either fold this into DR-0001 or record it separately — to be confirmed
  with the user when the records are written.

Three questions were put to the user before implementation began, and answered:

- **Tool installation.** Unpack the prebuilt trunk binary into the running
  container *and* record the same step in `.devcontainer/Dockerfile`, so it
  survives a rebuild.
- **Verification.** Everything checkable over HTTP is done here; the final
  browser eyeball is handed to the user.
- **Decision Records.** Two records, so a future 0.9 migration can supersede the
  version decision alone. This settles the open question above.

### 2026-08-02 (implementation)

Everything in the Plan was carried out as written. No step was superseded.

**Toolchain.** `CARGO_HOME` is `/usr/local/cargo`, not `~/.cargo` — the first
install attempt wrote to the wrong path and failed. `/usr/local/cargo/bin` is
world-writable in this image, so trunk 0.21.14 was unpacked there, and the same
step was added to the Dockerfile with the version pinned via an `ARG`.

`wasm-bindgen-cli` turned out not to need installing at all. On the first
`trunk build`, trunk logged `downloading wasm-bindgen version="0.2.126"` and
fetched a CLI matching the version resolved in `Cargo.lock` into its own cache.
This is the reason the notorious wasm-bindgen mismatch happens when the CLI *is*
installed by hand — recorded in DR-0001.

**Versions confirmed on crates.io.** leptos 0.8.20, leptos_router 0.8.15,
axum 0.8.9, tokio 1.53.1, serde 1.0.229, gloo-net 0.7.0, trunk 0.21.14,
wasm-bindgen 0.2.126. `leptos 0.9.0-beta` and `trunk 0.22.0-beta.2` exist and
were not used.

Note for anyone querying crates.io directly: the API rejects curl's default
user agent and returns an empty body, which reads like a network failure. Send
an explicit `User-Agent`, or use `cargo info`.

**`leptos_router` 0.8 has no `csr` feature** — only `ssr`, `nightly`, and
`tracing`. A CSR build depends on it with default features; only `leptos` itself
takes `features = ["csr"]`. Recorded in DR-0002.

**Layout.** Because the workspace root manifest is virtual, trunk cannot target
it. The root `index.html` points `data-trunk rel="rust"` at
`crates/app/Cargo.toml` instead, which is also what lets `style/` and `public/`
sit at the root as the Clarifications required.

**Data fetching** uses `LocalResource` rather than `Resource`: the browser fetch
future is not `Send`, and a CSR build has no server to run it on.

**Recipe naming.** `.claude/settings.json` denies `just run`, so the dev servers
are `just dev-web` and `just dev-api`, started by the user.

Decision Records written: **DR-0001** (CSR SPA with a separate API) and
**DR-0002** (staying on the Leptos 0.8 line). DR-0001 also carries the pitfalls
that have no home in a Design Document: the trunk↔wasm-bindgen coupling, the
gap between the dev proxy and production CORS, and the SPA-fallback requirement
that CloudFront will have to reproduce.

### 2026-08-02 (closing out)

Reviewing the diff against the plan turned up three declarations that nothing
used: the `signal` feature on tokio, `serde_json` in `[workspace.dependencies]`,
and path entries for `app` and `server` that no crate depends on. All removed;
`cargo check` for both targets still passes. Unused entries in a manifest that
exists to be the single statement of what is depended on are worse than noise.

The user confirmed the browser checks pass, and confirmed the three Design
Documents.

`GET /health` was implemented but appeared in no Design Document — the backend
document is deliberately unwritten, so the endpoint had nowhere to live.
`docs/design/index.md` now states the API's whole present surface in the place
that explains why the backend document does not exist yet.

Written as **DR-0003**: trunk installed as a pinned prebuilt binary, with
`wasm-bindgen-cli` deliberately left to trunk. The reasoning had been sitting in
a Dockerfile comment and a footnote to DR-0001, neither of which records the
alternatives (`cargo install` from source, `cargo-binstall`, ad-hoc installs) or
the reason pinning the CLI by hand is the cause of the mismatch rather than the
cure. The crates.io user-agent trap is recorded there too — it had no other
durable home.

## Verification

- `cargo check --workspace` and `cargo clippy --workspace` pass without warnings
- `trunk build` succeeds and emits WASM and HTML into `dist/`
- `cargo run -p server` starts axum, and `curl` returns the expected JSON
- With both running, the `trunk serve` page renders data from the API and the
  browser console is free of errors
- Routing works: navigation between routes, and routes still render after reload

### Results, 2026-08-02

Passed:

- `cargo fmt --all --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo clippy -p app --target wasm32-unknown-unknown -- -D warnings` — clean.
- `trunk build` — `dist/` holds `index.html`, a 2.9 MB debug `.wasm`, the JS
  glue, the hashed CSS, and `public/favicon.svg`.
- `curl localhost:3000/health` → `ok`;
  `curl localhost:3000/api/greeting` → `{"message":"Hello from axum."}`.
- Through `trunk serve` on :8080: `/` returns the built HTML,
  `/api/greeting` returns the same JSON **via the proxy**, and `/about` returns
  `index.html` — the SPA fallback that makes a reload on a route work. The
  `.wasm`, the CSS, and the favicon all return 200.

Confirmed in the browser by the user, with `just dev-api` and `just dev-web`
running:

- [x] `http://localhost:8080/` renders the API greeting, console clean
- [x] navigation to `/about` and back works
- [x] reloading while on `/about` still renders

Every item in this section has passed.

## Retirement

- [x] Design Documents updated — `docs/design/index.md`, `frontend.md`, and
      `workspace.md`, confirmed by the user on 2026-08-02.
- [x] Decision Records written — DR-0001 (CSR architecture),
      DR-0002 (Leptos 0.8 line), DR-0003 (toolchain installation)
- [x] Non-obvious knowledge preserved — rejected alternatives, pitfalls hit, and
      non-obvious constraints: the trunk / wasm-bindgen coupling and the
      crates.io user-agent trap in DR-0003, the dev-proxy-versus-production-CORS
      gap and the SPA-fallback requirement in DR-0001, `leptos_router` having no
      `csr` feature in DR-0002.
- [x] No durable document depends on this log — `docs/design/` and
      `docs/decisions/` were grepped for this file and for the plan file; no
      hits.

The checklist is satisfied. At the user's request this log and its plan file
stay in the working tree until the work is committed, and are deleted after
that; version control keeps them either way.
