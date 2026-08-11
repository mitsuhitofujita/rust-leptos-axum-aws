# A local stand-in for the API's edge

Status: in progress
Started: 2026-08-11
Branch: main

## Request

Before the work in `docs/work/2026-08-10-api-artefact-packaging.md` is attempted,
establish a way to verify the system locally. The two parts of the deployed edge
that `crates/server` is written against but never exercises outside AWS are the
DynamoDB table and the authentication filter — API Gateway's JWT authorizer
together with the request context the Lambda Web Adapter forwards. Both are to be
reproduced on a developer's machine, following whatever is the best practice for
doing so.

The result is meant to be used for a long time, so it is to be built to last
rather than as a throwaway. Anything that has to be written is written in Rust:
this is a devcontainer and Python is not available. `ripgrep` is.

The four-phase shape proposed in conversation is accepted and all four phases are
to be carried out. The work may be split into Work Logs as seems best, ordered so
that each piece can be carried out on its own and the sequence can be abandoned
part way through, smallest first.

**This log answers the second phase: the authentication filter and the rest of
the edge.** The others are `2026-08-11-local-dynamodb.md`,
`2026-08-11-local-token-verification.md` and
`2026-08-11-end-to-end-verification.md`. This phase does not depend on the first
— the stand-in works against either store — and the third extends what this one
builds.

## Interpretation

**What is being asked.** Everything between the browser and `crates/server` in a
deployed request exists only in `infra/api/apigateway.tf` and in AWS. Five
behaviours live there and are unobservable locally:

| Behaviour | Where it is decided |
| --- | --- |
| Only the methods in `local.api_methods` exist under `/api`; anything else is a 404 | `apigateway.tf` |
| `OPTIONS` is answered by the HTTP API itself, ahead of the authorizer | DR-0009 |
| A request without a valid token is refused with 401 before the function is invoked | DR-0010 |
| The authorizer's claims arrive as the `x-amzn-request-context` header | DR-0017 |
| Claims are a map of strings, whatever the token held | payload format 2.0 |

The answer is a small reverse proxy that plays both API Gateway and the Lambda
Web Adapter, sitting in front of the unmodified service. It reproduces the edge;
it is not a second implementation of anything the service does.

**Why it goes outside `crates/server`.** In the deployment it is outside. DR-0017
rests on `crates/server` being an ordinary axum binary that knows nothing about
Lambda, and a stand-in that had to be compiled into the service would destroy the
property it exists to check.

**The two behaviours that must not be approximated.**

- The stand-in must **discard any inbound `x-amzn-request-context` and always
  write its own**. Production's safety argument is that API Gateway overwrites
  the header on every request (DR-0017); a rig that passes a client's copy
  through would be a mirror in which the header is forgeable, and would teach the
  opposite of what is true.
- Claims must be **stringified**, as API Gateway stringifies them. `identity.rs`
  deserialises `HashMap<String, String>`, so a single non-string claim fails the
  decode of the whole context, `subject()` returns `None`, and the request is
  silently attributed to the development owner. That failure is invisible today,
  in every environment. Pinning it is one of the main reasons to build this.

**Out of scope.**

- Verifying real tokens. This phase decodes without verifying; the third phase
  adds verification against the pool's JWKS.
- The store. Either one works behind this.
- Being a general API Gateway emulator. It reproduces this project's edge, and
  its route table is this project's `local.api_methods`.
- Any change to `crates/app` or to the sign-in flow.

**Assumptions.**

- Adding a development-only crate to the workspace is acceptable; `crates/icongen`
  is the precedent and `workspace.md` already describes that category.
- The default mode must leave a developer who wants none of this exactly where
  they are today — DR-0008's principle again.
- Three terminals rather than two is an acceptable cost when the rig is in use.

## Plan

1. **`crates/devgateway`**, a development-only binary crate. New workspace
   dependencies: an HTTP client for the forwarding leg, built without OpenSSL so
   the image needs no new system packages. `axum`, `tokio`, `serde` and
   `serde_json` are already declared.
2. **The route table**, mirroring `local.api_methods`: `GET` and `POST` under
   `/api`, plus `GET /health`. Anything else answers `404 {"message":"Not
   Found"}`, which is what an HTTP API answers for an unmatched route — not a
   405, which is what a naive stand-in would produce and would hide the mismatch
   this is meant to expose.
3. **`OPTIONS`** answered by the stand-in from a CORS configuration mirroring
   `cors_configuration`, never forwarded and never authorized.
4. **Two authorizer modes.** `passthrough` forwards untouched, which is exactly
   today's `just dev-api`. `local`, the default, requires an `Authorization`
   header on `/api`, decodes a JWT without verifying it — or falls back to a
   configured subject when the token is not one — and answers `401
   {"message":"Unauthorized"}` when there is nothing to read.
5. **The request context.** Discard any inbound copy unconditionally. Build a
   payload-2.0-shaped context, claims stringified, and attach it as
   `x-amzn-request-context`. `/health` gets a context with no `authorizer` member
   at all, which is the shape `identity.rs` already has a test for.
6. **Ports.** The stand-in on 3001, the service unchanged on 3000, and
   `Trunk.toml`'s proxy moved to 3001. `just dev-api` keeps working on its own,
   so the no-stand-in path survives.
7. **`just dev-gateway`**, and a note in the recipe about which mode is which.
8. **Tests in the crate**: the route table, the 404-not-405 distinction, the
   preflight answer, the inbound header being discarded, and the stringified
   claims. These are unit tests over the request-building code; the end-to-end
   version is the fourth phase.
9. **Documents.** A Decision Record for reproducing the edge outside the service
   rather than inside it. Draft updates to `workspace.md` (the crate, the
   recipes), `deployment.md` (that the edge's behaviour is mirrored locally, and
   where the mirror is) and `frontend.md` (the proxy target), for confirmation.

## Progress

To be appended.

## Verification

To be recorded. The intended checks: `cargo test -p devgateway`; a `DELETE` to
`/api/action-types` answering 404 rather than reaching the service; a request
with no `Authorization` answering 401; a request carrying a forged
`x-amzn-request-context` and no token answering 401 rather than being attributed
to the forged subject; and `just dev-web` still serving the SPA through the
proxy.

## Retirement

- [ ] Design Documents updated — `workspace.md`, `deployment.md`, `frontend.md`
- [ ] Decision Records written (DR-____) — the edge is reproduced outside the
      service, in a development-only crate
- [ ] Non-obvious knowledge preserved — that a non-string claim silently
      degrades to the development owner; that an unmatched route is a 404 and not
      a 405; that discarding the inbound header is what keeps the rig honest
      about DR-0017
- [ ] No durable document depends on this log
