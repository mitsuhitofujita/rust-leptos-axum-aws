# Design Document Index

Updated: 2026-08-02

The entry point to the durable layer. Start here, read the document covering the
area being changed, and follow the Decision Record citations when a constraint
looks arbitrary. See `docs/README.md` for how these documents relate to Work
Logs and Decision Records.

## The system in one paragraph

A web application: a client-side rendered Leptos single-page application,
compiled to WebAssembly and delivered as static files, talking to a separate
axum HTTP API. Both are Rust crates in one Cargo workspace, sharing the types
that cross the boundary. AWS is the intended deployment target.

## Documents

| Document | Covers |
| --- | --- |
| [workspace.md](workspace.md) | Crate layout, dependency management, toolchain, task runner |
| [frontend.md](frontend.md) | The Leptos SPA: routing, data fetching, build, assets |

Not yet written, because it does not exist yet:

- **backend** — the axum service in `crates/server` holds no domain logic. Its
  whole surface is two endpoints that exist to give the frontend something real
  to call: `GET /health`, returning `ok`, and `GET /api/greeting`, returning a
  `shared::Greeting` as JSON. It binds `127.0.0.1:3000` and configures no CORS,
  for the reason given in DR-0001. A document belongs here once the service
  does something.
- **deployment** — S3, CloudFront, the API's runtime, IaC, CI/CD

## Decision Records

| Record | Decision |
| --- | --- |
| [DR-0001](../decisions/DR-0001-csr-spa-with-separate-api.md) | CSR SPA served as static files, with a separate API |
| [DR-0002](../decisions/DR-0002-stay-on-the-leptos-0-8-line.md) | Build on the Leptos 0.8 line, not the 0.9 prerelease |
| [DR-0003](../decisions/DR-0003-trunk-as-a-pinned-prebuilt-binary.md) | trunk as a pinned prebuilt binary; wasm-bindgen left to trunk |
