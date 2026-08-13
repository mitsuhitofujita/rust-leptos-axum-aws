# Workspace

Updated: 2026-08-13

Note: `crates/devgateway`'s reduction to the thin adapter below is decided
(DR-0023) and not yet carried out. The crate still holds the `local` and
`passthrough` modes, the route table and the context builder that DR-0021 gave
it. This document states the intended design.

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
  devgateway/         stands in for the deployed edge locally; ships nothing
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

`crates/devgateway` is the second of that kind, and it does one thing: it
verifies a real Cognito token the way the deployed authorizer verifies it, and
converts it into the `AuthContext` the service reads (DR-0022, DR-0024). It sits
in front of the unmodified service, forwarding what it accepts and refusing what
it does not.

It is deliberately not a local API Gateway. It has no route table, answers no
preflight, and does not reproduce the edge's behaviour in any other respect —
that fidelity was tried, under DR-0021, and retracted as the wrong cost to carry
against a specification AWS holds and can change (DR-0023). What survives from
that decision is where the crate lives: **outside `crates/server`**, because in
the deployment the component it stands in for is outside the service. It depends
on nothing in the workspace — least of all on `crates/server`, which is the
point — and nothing depends on it.

Its three third-party dependencies beyond what the service already uses are
`hyper-util` for the forwarding leg, and `hyper-rustls` and `aws-lc-rs` for the
JWKS fetch and the signature check. All three were already in `Cargo.lock` — the
first underneath axum, the other two underneath `aws-sdk-dynamodb` — so declaring
them adds dependency edges and no packages, and the devcontainer image needs
nothing new. `aws-lc-rs` is declared without default features, because `ring-io`
and `ring-sig-verify` are a compatibility surface for code ported from `ring` and
would have been the one thing to add a package.

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

**DynamoDB Local** is unpacked into `/opt/dynamodb-local` in the same image, from
a date-pinned archive checked against AWS's published SHA-256, and the
`default-jre-headless` package installed beside it is there to run that jar and
for nothing else. It is what the DynamoDB half of `crates/server` is verified
against without a deployment — DR-0020.

**Tasks** (`just <recipe>`):

| Recipe | What it does |
| --- | --- |
| `dev-web` | `trunk serve` — frontend on :8080, proxying `/api` to :3000 |
| `dev-web-auth` | the same dev server with sign-in switched on; resolves the two Cognito values from SSM, so it needs AWS credentials |
| `dev-web-gateway` | the same dev server with `/api` proxied to :3001 instead, so it goes through `dev-gateway` |
| `dev-api` | `cargo run -p server` — API on :3000, on the in-memory store, on mock authentication |
| `dev-gateway` | `cargo run -p devgateway` — the token adapter on :3001, forwarding to :3000; resolves the issuer and app client id from SSM, so it needs AWS credentials |
| `dynamo` | DynamoDB Local on :8000, in memory, `-sharedDb`, and reporting nothing to AWS |
| `dynamo-stop` | stop it, for when Ctrl-C in its own terminal is not available |
| `dynamo-table` | create the local table, idempotently; needs `dynamo` running |
| `dev-api-dynamo` | the same API server pointed at `dynamo` instead of at its in-memory store |
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

`dynamo`, `dynamo-table` and `dev-api-dynamo` are an opt-in verification mode
beside that default rather than a replacement for it: they run the store the
deployed function uses, on a machine with no AWS credentials, and nothing about
a fresh clone changes if they are never used (DR-0020). The credentials those
recipes pass are fake on purpose — a process that cannot authenticate anywhere
cannot reach the real table by accident.

`dev-gateway` and `dev-web-gateway` are the second such mode, and they answer one
question: would the deployed authorizer accept this token? `dev-gateway` sits
between the two and verifies a real Cognito token against the pool's published
keys and against the `issuer` and `audience` in `infra/api/apigateway.tf`, so a
wrong one is visible before an apply rather than as a 401 after it (DR-0022). It
is the one recipe here that needs AWS credentials and the network. Three
terminals when it is in use, and two when it is not.

It answers nothing else. The route table, the preflight, the 404 for an unrouted
method — everything a deployment adds between the browser and the service beyond
the authorizer's verdict — is verified against real AWS instead, because
reproducing it locally means maintaining a second telling of AWS's specification
in this repository (DR-0023).

**Two callers are a property of `dev-api`, not of the adapter.** Mock
authentication takes a subject, so `just dev-api` can be one caller or another
without a token, an adapter or AWS credentials, and the isolation
`identity::Owner` provides stays checkable on a machine with none of them
(DR-0024). An unnamed subject is the constant development owner, which is what
keeps a fresh clone working with no configuration at all (DR-0018).

The `/api` proxy target lives in these recipes rather than in `Trunk.toml`,
because there are two things it can point at — see the constraint below.

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
- **`Trunk.toml` holds no `[[proxy]]` block; each `dev-web*` recipe passes
  `--proxy-backend`.** There are two backends the dev server can want, and trunk
  *appends* a command-line backend to the file's entries rather than overriding
  them, so a default in the file could not be overridden and two entries would
  both claim `/api`. The cost is that a bare `trunk serve` outside `just` no
  longer proxies — DR-0023.
- **`crates/devgateway`'s audience rule is a hand-maintained copy of API
  Gateway's behaviour**, not of its configuration: the app client id is matched
  against `client_id` or `aud` because that is what a JWT authorizer does, and
  nothing here would notice AWS changing it — DR-0022. It is the one such copy
  the workspace keeps, and DR-0023 names it as a deliberate exception rather than
  a precedent, because what it guards is a `jwt_configuration` fault this
  repository owns.
- **`dev-gateway`'s two values have no defaults**, which is the one place this
  workspace departs from "an unset value means something workable". A defaulted
  issuer would verify against the wrong pool and a defaulted audience would
  accept what the deployment refuses; both look exactly like the misconfiguration
  the adapter exists to catch, so an unset one stops the process at startup —
  DR-0022.
- **`dev-gateway` cannot be two callers.** It verifies tokens, and there is
  nothing to verify a bare name against, so comparing two partitions is
  `dev-api`'s mock authentication rather than anything the adapter offers —
  DR-0024.
- `just dynamo` must keep `-sharedDb`. Without it DynamoDB Local keeps a
  separate database per access key and region, so `dynamo-table` and
  `dev-api-dynamo` would create and query two different tables, both would
  succeed, and nothing would report it — DR-0020.
- The image has no process tools. `ps`, `pgrep`, `pkill`, `fuser`, `lsof`, `ss`
  and `netstat` are all absent, so anything that has to find a running process
  reads `/proc` directly, as `dynamo-stop` does. A shell may define `pkill` as a
  function, which makes it look present until it is run.
- A recipe's comment is not free-form. `just --list` shows only the last comment
  line before a recipe, so each carries its explanation, then a blank line, then
  a one-line summary; an explanation without that break is listed as a fragment
  ending mid-sentence.
- Checking the workspace for the host target is not sufficient. `crates/app`
  must also be checked for `wasm32-unknown-unknown`; `just check` and `just
  lint` do both.
- The devcontainer has no browser and no Node.js. Anything the SPA does in a
  DOM — the sign-in redirect, the header control, the headers on a request —
  can only be checked by running `dev-web` or `dev-web-auth` and opening the
  forwarded port from outside the container. `check`, `lint` and `build` reach
  the compilation of that code and nothing further.
