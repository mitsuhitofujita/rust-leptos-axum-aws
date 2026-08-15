# Reducing crates/devgateway to the thin adapter

Work Log: `docs/work/2026-08-14-reducing-the-devgateway.md`

## Context

DR-0023 retracted DR-0021's local reproduction of the deployed edge. Of the six
behaviours `crates/devgateway` reproduces, exactly one survives: the authorizer's
actual verdict against a real Cognito token (DR-0022). The crate is to become "a
thin adapter that converts a verified JWT into an `AuthContext`, and does nothing
else".

`docs/design/workspace.md` and `docs/design/deployment.md` already describe that
reduced crate as the current design — `workspace.md` carries a dated note saying
so and admitting the code has not caught up. The `AuthContext` half of the work
is done (`docs/work/2026-08-14-introducing-the-authcontext.md`): the service
reads `x-auth-edge` and `x-auth-subject` and the adapter already writes them.
What remains is deleting everything else the crate does, so that the documents
become true and the two hand-maintained copies of AWS's specification — the route
table and the CORS configuration — stop existing.

This is the second and last follow-on that
`docs/work/2026-08-13-local-development-policy.md` deferred; retiring it unblocks
that log.

**Settled with the user:** every request through the adapter is authorized,
whatever its path. No exemption is kept for `/health`, so nothing of the route
table survives. `GET /health` through :3001 becomes a 401 where it is currently
`ok`; nothing uses it, since trunk proxies only `/api` (`gateway_backend` in the
`justfile`) and the probe exists for the deployed target.

## Approach

### 1. `crates/devgateway/src/config.rs`

Delete `Mode`, `mode()`, `DEVGATEWAY_MODE`, `allow_origin`, `ALLOW_ORIGIN` and
`DEVGATEWAY_ALLOW_ORIGIN`. `Config` becomes `{ address, upstream, verification:
Verification }` — `verification` is no longer an `Option`, so `required()` is the
only path and `var()` serves `address` and `upstream` alone.

Rewrite the module documentation. Its opening claim — every value has a default,
so a bare `cargo run -p devgateway` is the useful configuration — stops being
true of the crate, and what was the `cognito`-mode exception (DR-0022: a
defaulted issuer or audience is indistinguishable from the misconfiguration the
adapter exists to catch) becomes its only rule.

`Config::for_test` loses its `Mode` argument and always carries the
`testkey::ISSUER` / `testkey::AUDIENCE` fixture.

### 2. `crates/devgateway/src/authorizer.rs`

Delete the unverified path: `decoded()`, `claims()`, and `Bearer alice` as a
caller named alice. `authorize(parts, verifier: &Verifier)` takes a reference
rather than an `Option` and has one path — `bearer()`, then `verified()`, then
`report()` or the printed reason plus `Authorization::Refused`.

Rewrite the module documentation, which is organised around the two behaviours
and their difference.

Tests: delete `reads_the_claims_of_a_jwt_without_verifying_it`,
`accepts_a_jwt_whose_signature_is_nonsense`,
`a_bearer_value_that_is_not_a_jwt_is_the_subject_itself`, and the `local`-mode
assertion at the end of `a_token_whose_signature_was_tampered_with_is_refused`.
`accepts_a_token_without_the_bearer_prefix` moves to a signed fixture token.
Keep every verification test unchanged, including
`bearer_alice_is_refused_in_cognito_mode` — with `local` mode gone it stops being
a comparison and becomes the plain statement that a bare name is not a token.
`allowed()` and its `None` verifier go.

### 3. `crates/devgateway/src/edge.rs`

Reduce to strip, authorize, attach, refuse. Delete `Route`, `route()`,
`API_METHODS`, `preflight()`, `allow_origin()`, `ALLOW_HEADERS`, `MAX_AGE`, and
the `Mode` handling; no path or method is examined at all.

```rust
pub fn decide(verifier: &Verifier, parts: &mut Parts) -> Outcome
```

`Outcome::Forward` loses its `Option<HeaderValue>` payload. Keep `attach()`
verbatim — it is the conversion the crate exists for, and its comment about the
edge header going on unconditionally is the DR-0025 argument. Keep `answer()`
for the 401, which must stay byte-identical to what the deployed authorizer
answers (DR-0022). Keep the unconditional removal of `AUTH_HEADERS` before
anything else looks at the request, and its comment.

Rewrite the module documentation, which opens "The route table, the preflight
answer, and the one decision the stand-in makes about a request."

### 4. Tests in `edge.rs`

`send()` loses its `Mode` argument and always builds the fixture verifier from
`testkey::JWKS`, which is the largest mechanical change here: the surviving tests
move from `jwt()`-built unverifiable tokens to `testkey::sign`. Add a local
helper mirroring `authorizer`'s `access_token()`, signing with
`unix_time()`-based `exp` so the real clock inside `authorize` accepts it.

Keep, adapted: `a_request_under_api_without_a_token_is_refused`,
`a_forged_context_does_not_survive_a_missing_token`,
`a_forged_context_is_replaced_when_a_token_is_present`,
`an_accepted_token_with_no_subject_is_marked_and_unnamed` (sign a payload with
`iss`, `client_id` and `exp` but no `sub`), and
`a_refusal_in_cognito_mode_says_no_more_than_the_deployed_one` — renamed, since
there is no other mode to distinguish it from.

Delete: the two route-table tests, `the_probe_is_forwarded_without_a_token_...`,
`a_forged_context_is_stripped_from_the_probe`, the three preflight tests,
`the_token_cognito_mode_refuses_is_the_one_local_mode_forwards`, and
`passthrough_adds_nothing_and_removes_nothing`.

Add one test asserting a path outside `/api` — `/health` — is authorized like
any other. It is the behaviour the clarification chose, and with no exemption in
the code nothing else would record it.

### 5. `main.rs` and `proxy.rs`

`main.rs`: `Edge` becomes `{ upstream: String, verifier: Verifier, forwarder }`,
built by moving `verification` out of `Config` into the `Verifier` before the
listener binds — the existing ordering comment holds and is kept. `announce()`
loses its `Mode` match and keeps the issuer/audience echo and the key count,
which is the cheapest half of what DR-0022 checks. The module documentation lists
four reproduced behaviours and the note that only one survives DR-0023; it
becomes a description of the one.

`proxy.rs`: `forward()` loses its `allow_origin` parameter and the
`access-control-allow-origin` insertion; `HeaderValue` leaves its imports. The
module's opening line still names the request context and is updated. Everything
else — hop-by-hop stripping, the `host` removal, `unreachable_upstream` — is
untouched.

`Cargo.toml` is unchanged: `serde`, `serde_json` and `base64` are all still used
by the verification path.

### 6. `justfile`

Delete `dev-gateway-cognito` and make `dev-gateway` what it was — the
`DEVGATEWAY_ISSUER` / `DEVGATEWAY_AUDIENCE` resolution from SSM via `just _ssm`.
Rewrite the section comment (currently the three modes and the retracted rig) and
the `dev-web-gateway` comment that names both recipes. Observe the rule
`workspace.md` records: explanation, blank line, then the one-line summary
`just --list` shows.

The comment should state what the reduced adapter is for and the two things a
developer will otherwise be surprised by — every path needs a token, `/health`
included, and a `dev-web` bundle sends no `Authorization` header at all, so
`dev-web-auth` is what pairs with it.

### 7. Design Documents (drafted, then confirmed before the work is complete)

- **`workspace.md`** — remove the dated note at the top. Check each sentence
  about the adapter against the built code, particularly the crate-list line
  ("stands in for the deployed edge locally"), the `dev-gateway` recipe row, and
  the four constraints that mention it. Add one constraint: the adapter
  authorizes every path where the deployed edge routes `/health` outside the
  authorizer, and nothing in the crate shows the divergence, because what would
  show it is an exemption and there is none.
- **`deployment.md`** — already describes `just dev-gateway` as the thin adapter
  with the authorizer's verdict. Verify rather than assume, especially the
  browser-through-the-adapter constraint around line 365.
- **`index.md`** — no change expected. It gains a row only if step 8 produces a
  record.

### 8. The `/health` divergence

Planned home is the `workspace.md` constraint above. A Decision Record is
warranted only if the reasoning turns out to need an Alternatives section;
decided once the code is written and the sentence can be checked against it. If
one is written it is DR-0026 and `index.md` gains its row.

### 9. Report back

Append to `docs/work/2026-08-13-local-development-policy.md`: both follow-on
pieces now exist and are complete, which is its last outstanding retirement item.

## Verification

- `just test` — the workspace suites. Expect `devgateway`'s count to drop
  substantially; the ones that remain are the verification set plus the two
  forgery tests, the marked-and-unnamed test, and the new any-path test.
- `just lint` (host and `wasm32-unknown-unknown`, warnings denied) and `just fmt`.
- `just dev-api` in one terminal, `just dev-gateway` in another — needs AWS
  credentials for SSM and the network for the JWKS fetch — and through :3001:
  - a real Cognito access token from `dev-web-auth` on `/api/action-types`
    returns that user's items and the adapter prints one `200 — verified access
    token for sub=…` line;
  - no `Authorization` header is `401 {"message":"Unauthorized"}`;
  - `Bearer alice` is a 401, with the reason on the adapter's terminal and not in
    the response;
  - forged `x-auth-edge` / `x-auth-subject` sent *with* a valid token return the
    token's own items, not the forged subject's;
  - `GET /health` is a 401 — the intended change, checked rather than assumed.
- `just dev-web-gateway` with a `dev-web-auth`-style bundle for the browser leg,
  which is the only way to exercise the SPA against the adapter (the devcontainer
  has no browser; the port is opened from outside).
- Nothing in `infra/` changes, so no Terraform check is needed and no apply is
  implied.
