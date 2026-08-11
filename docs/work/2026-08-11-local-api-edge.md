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

### Clarifications

The stand-in's port must not become a requirement of the ordinary development
session. `Trunk.toml`'s `[[proxy]]` block is removed and each `just` recipe names
the backend it wants instead, so `dev-web` and `dev-web-auth` keep proxying to
the service on :3000 and a new recipe proxies to the stand-in on :3001. The cost
— that a bare `trunk serve` run outside `just` no longer proxies `/api` — is
accepted.

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
6. ~~**Ports.** The stand-in on 3001, the service unchanged on 3000, and
   `Trunk.toml`'s proxy moved to 3001. `just dev-api` keeps working on its own,
   so the no-stand-in path survives.~~ — **superseded**, see 6a. Moving
   `Trunk.toml`'s proxy would have left `just dev-web` unable to reach anything
   without the stand-in running, which is the opposite of the assumption above
   it. trunk 0.21.14 *appends* `--proxy-backend` to the `[[proxy]]` entries
   rather than overriding them (`src/cmd/serve.rs:168` in that release), so
   pointing the file at 3001 and overriding it per recipe is not available
   either: two entries would both claim `/api`.
6a. **Ports.** The stand-in on 3001, the service unchanged on 3000, and
   `Trunk.toml`'s `[[proxy]]` block removed. `dev-web` and `dev-web-auth` pass
   `--proxy-backend` for 3000, and a new `dev-web-gateway` passes it for 3001,
   so the two-terminal default survives untouched.
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

### 2026-08-11

**`crates/devgateway` written and working.** Six modules: `config` (the
environment and every default), `edge` (the route table, the CORS answer, and one
decision function), `authorizer` (the two modes), `context` (the payload-2.0
request context, claims stringified) and `proxy` (the forwarding leg). 19 unit
tests, all over a pure `edge::decide` rather than a router, so no socket and no
upstream are involved.

**The forwarding leg is `hyper-util`, and cost nothing.** It was already in
`Cargo.lock` underneath axum, so declaring it added feature flags and no
packages; the only new entry in the lock file is `devgateway` itself. Plain HTTP
to loopback means no TLS and therefore no OpenSSL, which is what plan step 1 was
guarding against. Phase three's JWKS fetch will need TLS, and `hyper-rustls` is
already in the lock for it.

**Step 6 was wrong and is superseded** — see the Clarifications above and step
6a. The finding worth keeping: trunk *appends* a command-line `--proxy-backend`
to `Trunk.toml`'s `[[proxy]]` entries rather than overriding them, so there is no
way to state a default in the file and override it per recipe. Both entries would
claim `/api`. The proxy target therefore lives in the recipes.

**Plan step 5's fallback subject was wrong, and the verification caught it.** As
planned, a bearer value that is not a JWT became a single configured subject
(`DEVGATEWAY_SUBJECT`), which meant `Bearer alice` and `Bearer bob` were *the same
caller* — the first run of the isolation check had both seeing both items. The
fallback now takes the bearer value itself as the subject, and
`DEVGATEWAY_SUBJECT` is gone. This is the difference between the rig being able to
express two callers and not, and the isolation check is the main thing it exists
for, so the flaw was worth the round trip. A real token that fails to decode lands
here too and becomes a subject that is the whole token — ugly in the partition,
which is the right kind of visible.

**Two things about the stand-in are exact rather than approximate**, and both are
recorded in DR-0021: the inbound `x-amzn-request-context` is discarded before
anything else looks at the request, and claims are stringified the way payload
format 2.0 stringifies them, including a list claim as `[a b]`.

**Decided, and recorded as DR-0021:** the edge is reproduced outside the service.
Alternatives weighed and rejected there: a mode inside `crates/server`, SAM or
LocalStack, `cargo lambda watch`, and an axum router as the route table.

## Verification

`just fmt-check`, `just check`, `just lint` and `cargo test -p devgateway` all
pass — 19 tests.

By hand, with `dev-api` and `dev-gateway` running:

| Check | Result |
| --- | --- |
| `DELETE /api/action-types` | `404 {"message":"Not Found"}`; the service is never reached |
| `GET /api/action-types` with no `Authorization` | `401 {"message":"Unauthorized"}` |
| A forged `x-amzn-request-context` and no token | 401, not the forged subject |
| A forged `x-amzn-request-context` *with* a token | forwarded with our context; the forgery is gone |
| `GET /health` | `ok`, unauthenticated, context carrying no `authorizer` |
| `OPTIONS` with the allowed origin | 204 with the four mirrored CORS headers, no token |
| A forwarded response with an `Origin` | carries `access-control-allow-origin` |
| `DEVGATEWAY_MODE=typo` | refused at startup rather than defaulted |

**Isolation, which nothing local could observe before.** `Bearer alice` and
`Bearer bob` each created an action type and each `GET` returned only its own.

**A token with non-string claims, end to end.** A hand-built JWT carrying `exp` as
a number, `email_verified` as a boolean and `cognito:groups` as a list reached the
service and its subject decoded — the silent failure named in the Interpretation,
now demonstrated not to occur through this path.

**`passthrough` shows the contrast.** The same forged header reaches the service
and wins, and `DELETE /api/action-types` gets a 405 from axum rather than a 404.
That is what `just dev-api` alone already is.

**The default path is undisturbed.** `just dev-web` with only `just dev-api`
running serves the SPA on :8080 and its `/api` calls; trunk logs `proxying /api ->
http://127.0.0.1:3000/api`. `just dev-web-gateway` logs `-> :3001`, serves the
same SPA, answers 401 for an `/api` call with no token, and returns alice's items
for one carrying `Bearer alice`.

## Retirement

- [x] Design Documents updated — `workspace.md`, `deployment.md`, `frontend.md`,
      and the record's row in `design/index.md`
- [x] Decision Records written — DR-0021, the deployed edge is reproduced
      locally, outside the service
- [x] Non-obvious knowledge preserved, in DR-0021: that a non-string claim would
      silently degrade to the development owner; that an unmatched route is a 404
      and not the 405 a router would give; that discarding the inbound header is
      what keeps the rig honest about DR-0017; and that trunk appends a
      command-line proxy backend rather than overriding the file's, which is in
      DR-0021's Consequences and in `workspace.md`'s constraints
- [ ] No durable document depends on this log — to be confirmed with the Design
      Document updates, which are drafted and awaiting review
