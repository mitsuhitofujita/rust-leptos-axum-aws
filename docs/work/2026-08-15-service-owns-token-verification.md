# Service owns token verification, and the API Gateway JWT authorizer is removed

Status: in progress
Started: 2026-08-15
Branch: main

## Request

Reopen the premise behind DR-0017, DR-0024 and DR-0025: that API Gateway's JWT
authorizer is the enforcement point in front of `crates/server`, and that
verifying a token inside the service would be a redundant re-implementation of
an authorizer that has already run. This time the authorizer is not assumed to
stay. `crates/server` is to verify Cognito access tokens itself — reusing the
JWKS fetch and RS256 verification already built and proven in
`crates/devgateway` under DR-0022 — and `aws_apigatewayv2_authorizer.cognito`
in `infra/api/apigateway.tf` is to be removed, so the same binary handles
authentication identically under `cargo run` and under Lambda.

The reason given: a web application that verifies its own tokens rather than
relying on a gateway's managed authorizer is not an unusual shape, and doing so
here is judged to cost less in total than the current split — the managed
authorizer, the two-integration parameter mapping (DR-0025), the enumerated
methods it requires instead of a single `ANY` route (DR-0009), and the gap
this opens in the Lambda Web Adapter's own premise: that the same binary
behaves the same way locally and deployed, which currently holds for the
request and response but not for who is allowed to make the request.

### Clarifications

Confirmed in conversation: this is not the case DR-0017 and DR-0024 already
refused twice. Both refusals assumed the authorizer stays in front of the
function and judged verifying inside the service redundant against it. Here
the authorizer is not assumed to stay at all; the service becomes the only
place verification happens, so there is nothing left for it to be redundant
with.

## Interpretation

**What is being asked.** An architecture change: move Cognito access-token
verification from API Gateway's managed JWT authorizer into `crates/server`
itself, and remove the authorizer once the service verifies on its own.

**What this reverses, precisely, and what it does not.**

- **DR-0017**, already superseded by DR-0024, is not reopened as a live
  decision — but its refusal of in-service verification is the refusal DR-0024
  repeats, and this work reopens that specific point on grounds neither record
  considered: no authorizer left to be redundant with.
- **DR-0024**'s "Verify the token in `crates/server`" alternative is reopened
  and, on the grounds above, answered the other way this time. DR-0024's
  placement of the AWS-to-`AuthContext` conversion "outside `crates/server`"
  is reversed by construction — the conversion moves inside, because there is
  no longer an edge component to hold it. DR-0024's stronger claim survives:
  `crates/server` still defines `AuthContext` and reads only that; nothing
  downstream of verification needs to change.
- **DR-0025** is reversed in full. Parameter mapping producing
  `x-auth-subject` and `x-auth-edge` has nothing left to feed once there is no
  authorizer output to map, and the two-integration split it required existed
  only to keep those mappings off `/health`.
- **DR-0009**'s reason for enumerating methods instead of a single `ANY` route
  — the authorizer intercepting `OPTIONS` ahead of the HTTP API's own
  preflight answer — goes away once nothing at the API Gateway layer
  authorizes anything. Whether `ANY` becomes viable, or the enumerated routes
  stay for some other reason, is for the Plan below to check rather than
  assume.
- **DR-0023**'s principle is not reversed and, on inspection, supports this
  direction rather than opposing it: fetching a real pool's real JWKS and
  verifying a real signature against it is the record's own example of
  something that is *not* AWS re-implementation, because it is the actual
  protocol rather than a second telling of AWS's behaviour. What moves is
  *where* that verification runs, not *whether* it re-implements anything.
- **DR-0022** and its Work Log
  (`docs/work/2026-08-11-local-token-verification.md`, still open) built and
  proved the JWKS-fetch-and-verify logic this work reuses, inside
  `crates/devgateway`. That log's own Interpretation says explicitly that
  verifying there "does not contradict DR-0017's refusal... that refusal is
  about the service; this is the thing standing in for the component whose
  job verification is." Once verification moves into the service, `devgateway`
  is no longer standing in for anything — its reason to exist is answered by
  this work, not preserved by it. That log is cross-referenced here rather
  than edited; whether and how it closes is a `work-done` question once this
  is implemented.

**Out of scope.**

- DooD / the devcontainer's container-engine access — a separate, unrelated
  unit of work, tracked in `docs/work/2026-08-10-api-artefact-packaging.md`.
- Any change to how the SPA obtains or attaches a token (DR-0010) — invisible
  from `crates/app`, exactly as DR-0024 noted for its own change.
- Any change to Cognito itself, the `identity` Terraform layer, or the hosted
  UI.
- CI/CD.

**Assumptions.**

- The JWKS endpoint stays reachable from inside the Lambda function's network
  path — it is public, and `crates/devgateway` already fetches it from a
  devcontainer with no special networking, but this has not been checked from
  inside the Lambda execution environment specifically.
- `crates/devgateway`'s `jwks.rs` and the audience/issuer/expiry rule in
  `authorizer.rs` are the logic to reuse, not rewrite. Where they end up —
  moved into `crates/server`, or left in `crates/devgateway` and depended on —
  is a Plan question, not decided here.
- CORS preflight must keep working without a token once the authorizer is
  gone; the mechanism moves from "unrouted `OPTIONS` answered by the HTTP
  API" to whatever this work arranges instead, and has to be checked rather
  than assumed to carry over.

## Plan

1. **Locate the verification logic to reuse.** Read
   `crates/devgateway/src/authorizer.rs` alongside `jwks.rs` already read here,
   to see the full audience/issuer/expiry rule DR-0022 built, not just the key
   fetch and signature check.
2. **Decide where the logic lives.** Moved wholesale into `crates/server`, or
   factored so both `crates/server` and (temporarily, if kept) `devgateway`
   depend on it. `crates/shared` is not a candidate — DR-0024's Alternatives
   already ruled it out for carrying platform-specific dependencies into the
   WASM build.
3. **Add verification to `crates/server`.** An axum extractor or middleware
   that reads `Authorization: Bearer <token>`, verifies it against the pool's
   JWKS the way `crates/devgateway`'s `cognito` mode does, and produces the
   `AuthContext` DR-0024 already defined — replacing the two-header
   `x-auth-subject`/`x-auth-edge` consumption DR-0025 built.
4. **Decide `OPTIONS`.** The extractor/middleware must not run ahead of a
   preflight request, or every preflight fails without a token. Check whether
   axum's own routing can exclude `OPTIONS` before the extractor runs, or
   whether it needs an explicit skip.
5. **`infra/api/apigateway.tf`:** remove `aws_apigatewayv2_authorizer.cognito`,
   the two-integration split and both `request_parameters` maps from
   DR-0025, and `authorization_type`/`authorizer_id` from the `api` route.
   Check whether `local.api_methods` and the enumerated routes still need to
   stay enumerated for CORS's sake, or whether a single `ANY` route now works
   — DR-0009's reason for avoiding `ANY` was the authorizer, which is gone.
6. **CORS.** `cors_configuration` on `aws_apigatewayv2_api.this` is unaffected
   in principle; confirm a preflight is still answered without reaching
   `crates/server` at all, or decide it is acceptable for it to reach the
   service and be answered there instead, and check which one actually
   happens.
7. **`crates/devgateway`.** Once `crates/server` verifies tokens itself,
   decide whether `devgateway` still has a reason to exist. If it does not,
   retiring it is part of this work, not a separate cleanup; if it does (some
   purpose not yet identified), say what that purpose is.
8. **Verify against real AWS.** A request with no token is refused by the
   service itself, not by API Gateway; a valid token is accepted and
   attributed to its subject; a tampered or expired token is refused; a
   preflight succeeds with no token. All four were already checked against
   the *authorizer* by `docs/work/2026-08-11-local-token-verification.md`'s
   `cognito` mode — re-running them against the *service* is the equivalent
   check for the new arrangement.
9. **Documents.** `docs/design/backend.md` and `docs/design/deployment.md`
   rewritten for the new shape; a Decision Record superseding DR-0025 in
   full, and one narrowing DR-0024's placement claim — both written at
   `work-done` time, against what actually got built, not drafted ahead of
   it.

## Progress

### 2026-08-15
Log opened. Interpretation and Plan above are proposed; nothing implemented
yet.

## Verification

Not started.

## Retirement

- [ ] Design Documents updated
- [ ] Decision Records written (DR-____)
- [ ] Non-obvious knowledge preserved
- [ ] No durable document depends on this log
