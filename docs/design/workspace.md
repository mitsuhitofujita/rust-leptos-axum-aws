# Workspace

Updated: 2026-08-11

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

`crates/devgateway` is the second of that kind. It is an axum binary that plays
API Gateway and the Lambda Web Adapter in front of the unmodified service, so
that the route table, the preflight answer, the 401 and the
`x-amzn-request-context` header can be exercised without a deployment (DR-0021).
It depends on nothing in the workspace — least of all on `crates/server`, which
is the point — and nothing depends on it. Its one third-party dependency beyond
what the service already uses is `hyper-util`, for the forwarding leg; it was
already in `Cargo.lock` underneath axum, and it talks plain HTTP to loopback, so
no TLS stack and no system package came with it.

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
| `dev-api` | `cargo run -p server` — API on :3000, on the in-memory store |
| `dev-gateway` | `cargo run -p devgateway` — the edge stand-in on :3001, forwarding to :3000 |
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

`dev-gateway` and `dev-web-gateway` are the second such mode, and reproduce the
other half of what a deployment adds: `dev-gateway` sits between the two and
plays API Gateway and the Lambda Web Adapter, so `/api` needs an `Authorization`
header, a method outside the route table is a 404, `OPTIONS` is answered without
a token, and the request context arrives as the header the service reads
(DR-0021). Any bearer value works — a JWT is decoded without being verified, and
anything else is taken as the subject itself, so `Bearer alice` and `Bearer bob`
are two callers whose partitions can be compared. `DEVGATEWAY_MODE=passthrough`
turns all of it off, which is what `just dev-api` alone already is. Three
terminals when this is in use, and two when it is not.

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
  longer proxies — DR-0021.
- **`crates/devgateway`'s route table is a hand-maintained copy of
  `local.api_methods`** in `infra/api/apigateway.tf`, as `dynamo_table` and
  `project` are copies of Terraform values. A method added there has a second
  place to follow it, and a drift shows up the first time the stand-in is used
  and not before — DR-0021.
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
