# Workspace

Updated: 2026-08-10

## Purpose

Hold the frontend, the API, and the types they share in one Cargo workspace, so
that a change to the contract between them is a single compilation unit's
problem rather than a runtime surprise.

## Structure

```text
Cargo.toml            virtual workspace manifest; all dependency versions
rust-toolchain.toml   pinned toolchain and compilation targets
Trunk.toml            frontend dev server and build configuration
index.html            trunk's entry point (see frontend.md)
justfile              task runner
style/                plain CSS
public/               static assets copied verbatim into the bundle
crates/
  app/                the Leptos SPA — compiled to wasm32-unknown-unknown
  server/             the axum API — compiled to the host target
  shared/             types crossing the boundary
  icongen/            generates the action-type icon catalog; ships nothing
docs/                 this documentation (see docs/README.md)
```

`crates/shared` is depended on by both `app` and `server`; nothing else depends
on anything else. `shared` must therefore stay free of platform-specific
dependencies, since it is compiled for WASM and for the host alike.

`crates/icongen` is a developer tool that happens to live in the workspace. It
is run by hand, by `just icons`, and writes two source files that are committed:
`crates/shared/src/icon_names.rs` and `crates/app/src/icon_catalog.rs`. Nothing
depends on it and it depends on nothing that reaches a binary — its one
dependency, `lucide-leptos`, has no feature enabled and compiles to an empty
library, and is declared only so that `Cargo.lock` pins the version and cargo
unpacks the source it reads (DR-0019).

Third-party versions are declared once, in `[workspace.dependencies]` in the
root `Cargo.toml`. Member crates use `dep.workspace = true` and add only the
features they need. Package metadata (edition, rust-version, license) comes from
`[workspace.package]` the same way.

## Interfaces

**Toolchain.** `rust-toolchain.toml` pins Rust 1.96.1 and requests both
`x86_64-unknown-linux-gnu` and `wasm32-unknown-unknown`, so rustup provisions
the WASM target on the first cargo invocation.

**trunk** is installed in the devcontainer image as a prebuilt binary, pinned by
an `ARG` in `.devcontainer/Dockerfile`. `wasm-bindgen-cli` is deliberately *not*
installed: trunk downloads a CLI matching the resolved `wasm-bindgen` version
into its own cache. Installing or pinning it by hand is what causes
version-mismatch failures — DR-0003.

**Tasks** (`just <recipe>`):

| Recipe | What it does |
| --- | --- |
| `dev-web` | `trunk serve` — frontend on :8080, proxying `/api` to :3000 |
| `dev-web-auth` | the same dev server with sign-in switched on; resolves the two Cognito values from SSM, so it needs AWS credentials |
| `dev-api` | `cargo run -p server` — API on :3000 |
| `build` | release build of the workspace and of the WASM bundle |
| `check` | `cargo check` for the host, plus `app` for WASM |
| `lint` | clippy for both targets, warnings denied |
| `fmt` / `fmt-check` | rustfmt |
| `test` | `cargo test --workspace` |
| `icons` | regenerate the action-type icon catalog from the pinned `lucide-leptos`, and format what it wrote |
| `clean` | `cargo clean` and remove `dist/` |

Development needs `dev-api` and `dev-web` running together, in two terminals.
Neither needs credentials or configuration: the API stores action types in
memory when `TABLE_NAME` is unset, and the SPA disables sign-in when the two
Cognito variables are (DR-0008, DR-0018).

The `justfile` also holds the `tf-*` recipes that apply the infrastructure and
the `deploy-*` recipes that push the artefacts. Both sets belong to
`deployment.md` and are described there, not here.

## Constraints

- `crates/shared` depends on `serde` only. Adding tokio, axum, or any web-sys
  dependency to it breaks the WASM build of `crates/app`.
- `crates/server` and `crates/shared` must not depend on Leptos. Keeping the
  framework confined to `crates/app` is what bounds the cost of a future 0.9
  migration (DR-0002). `crates/icongen` is the exception the rule tolerates: it
  reaches Leptos through `lucide-leptos`, compiles neither, and is not shipped.
- Nothing runs `just icons`. The generated files, the pinned `lucide-leptos`
  version and the category list in `crates/icongen` agree only because someone
  ran it after moving one of them — DR-0019.
- Checking the workspace for the host target is not sufficient. `crates/app`
  must also be checked for `wasm32-unknown-unknown`; `just check` and `just
  lint` do both.
- The devcontainer has no browser and no Node.js. Anything the SPA does in a
  DOM — the sign-in redirect, the header control, the headers on a request —
  can only be checked by running `dev-web` or `dev-web-auth` and opening the
  forwarded port from outside the container. `check`, `lint` and `build` reach
  the compilation of that code and nothing further.
