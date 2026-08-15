# Introducing the AuthContext

Work Log: `docs/work/2026-08-14-introducing-the-authcontext.md`

## Context

DR-0024 decided that `crates/server` should describe its caller with its own
`AuthContext` type rather than by parsing API Gateway's request context, and that
the conversion from AWS's shape belongs in front of the service. Nothing
implements it yet: `crates/server/src/identity.rs` still deserialises
`x-amzn-request-context` into a `RequestContext` shaped after `lambda_http`'s and
takes `sub` out of a `HashMap<String, String>`.

That typing is the defect. API Gateway's payload format 2.0 stringifies every
claim, so the map decodes only because of a property of a payload format the
service has no other reason to know. If one claim ever arrives as a number or a
list, serde fails to decode the whole context, `subject()` returns `None`, and
the request is attributed to the development owner instead of being refused — a
write into the wrong partition that nothing reports. DR-0023 retired the local rig
that made this observable, so the answer has to be structural.

**DR-0024 has a gap this plan closes.** It names the reduced `crates/devgateway`
as the converter, but that crate ships nothing (`workspace.md`), and in a deployed
request the only things in front of the service are API Gateway and the Lambda
Web Adapter — neither of which knows about an `AuthContext`. Implementing DR-0024
literally would leave every deployed request with no context, and therefore
attributed to the development owner: the very failure it exists to prevent.

The answer is API Gateway's own request parameter mapping. It is AWS's mechanism
rather than a re-implementation of AWS behaviour, so DR-0023 permits it; it puts
the conversion in the component already in front of the service, which is what
DR-0024 asks for; and because `$context.authorizer.claims.sub` is a single value
API Gateway resolves, the claims map disappears from both ends and the original
defect has nowhere left to live.

Scope is the `AuthContext` boundary only. Reducing `crates/devgateway` — its route
table, preflight and three modes — is the separate follow-on the policy log
deferred.

## Design decisions

**The wire is a scalar header, `x-auth-subject`.** The mapping value is exactly
`$context.authorizer.claims.sub`, a single documented expression with no
interpolation into static text, so it is certain to work. The service parses
nothing at all. This supersedes the JSON `x-auth-context` proposed in the Work
Log's step 1, which was written before I established that mapping is
integration-level and would have required interpolating a `$context` variable
into a larger string — support I cannot confirm offline and whose failure would
surface only at apply.

**A second header marks that the edge spoke.** `overwrite:header.x-auth-edge` is
set to a static value on the authenticated integration. Without it, a mapping that
failed to resolve would omit `x-auth-subject`, the service would read that as "no
context", and the request would land in the development owner's partition —
reintroducing the silent misattribution in the one place it matters. With it the
three cases are structural rather than inferred:

| Arrives | Owner |
| --- | --- |
| `x-auth-edge` and a non-empty `x-auth-subject` | that subject |
| Neither header | `DEVELOPMENT_OWNER` — a developer's own machine, DR-0018 |
| `x-auth-edge` with `x-auth-subject` absent or empty | nobody; `401` |

The cost is one static mapping and one field. It keeps everything about identity
on the wire and in the component in front, needs no new environment variable, and
is checkable by `cargo test` with no AWS.

**The integration is split in two.** `request_parameters` lives on
`aws_apigatewayv2_integration`, not on `aws_apigatewayv2_route` — the route only
has `request_parameter_key`, which is validation. Today one integration serves both
the `/api` routes and `/health`, and `/health` runs outside the authorizer, where
the mapping source cannot resolve and API Gateway would skip it, letting a
caller-supplied header through. A second integration for `/health` carrying
`remove:` for both headers makes that structurally impossible instead of harmless
by coincidence.

**Mock authentication is the header itself.** No configuration variable and no
mock mode: a developer sends `x-auth-subject` with `curl` to be one caller or
another, and a request carrying nothing is the development owner. This is a
narrowing of DR-0024's table, which says "Configuration"; it is flagged in the
Work Log's Interpretation rather than absorbed silently.

## Changes

### `crates/server/src/identity.rs`

The whole of the service-side change, and the crate's only AWS-shaped file.

- Add `AuthContext { subject: String }` and the two header constants. No `serde`
  derive is needed — nothing is parsed.
- Delete `RequestContext`, `Authorizer`, `Jwt`, the `HashMap` and `serde` imports,
  and the `REQUEST_CONTEXT` constant. Leaving any of them behind unused would be
  the same coupling with a warning attached.
- Replace `subject()` with a function returning a three-way outcome over the two
  headers, matching the table above.
- `Owner`'s `Rejection` stops being `Infallible` and becomes a type implementing
  `IntoResponse`, answering `401`. Handlers need no change: they destructure
  `Owner(owner): Owner` and axum rejects before the body runs.
- Rewrite the module documentation, which currently explains the adapter's header
  at length. `DEVELOPMENT_OWNER` and its rationale stay.

Tests — the four cases, of which the last two are what this work exists to make
expressible:

1. both headers present → that subject
2. neither header → `DEVELOPMENT_OWNER`
3. `x-auth-edge` with no `x-auth-subject` → rejected
4. `x-auth-edge` with an empty `x-auth-subject` → rejected

A fifth is worth having: `x-auth-subject` without `x-auth-edge` → the development
owner, since an unmarked request is a developer's however it is dressed.

### `crates/devgateway`

Only what the adapter hands the service changes. `edge.rs` keeps its structure,
its route table, its preflight and its three modes.

- **Delete `src/context.rs`** entirely — `stringify`, `flatten`, the payload-2.0
  base object and its tests. Its replacement is small enough to live in `edge.rs`:
  take `sub` from the claims `authorizer::authorize` returned and attach the two
  headers.
- `edge.rs`: `ADAPTER_HEADERS` becomes the two new headers, stripped on the way in
  so a caller's copy never survives — the property the two forgery tests assert,
  which carry over unchanged in intent. `Route::Health` now attaches nothing
  rather than a context without an `authorizer` member, mirroring the split
  integration. Drop `the_adapters_lambda_context_is_discarded_too`: the service no
  longer reads either `x-amzn-*` header, so stripping them means nothing.
- `authorizer.rs` is untouched except for the message in `decoded()`, which tells
  the developer the service will fall back to the development owner when a token
  carries no `sub`. Under the new rule that case is a refusal, and the line should
  say so.
- A token whose claims carry no `sub` attaches `x-auth-edge` with an empty
  `x-auth-subject`, so local behaviour matches the deployed rule rather than
  diverging from it.

### `infra/api/apigateway.tf`

- Rename `aws_apigatewayv2_integration.lambda` to `.api` and give it
  `request_parameters` with `overwrite:header.x-auth-subject` from
  `$context.authorizer.claims.sub` and `overwrite:header.x-auth-edge` as a static
  value.
- Add `aws_apigatewayv2_integration.health`, `AWS_PROXY` to the same
  `invoke_arn`, carrying `remove:` for both headers.
- Point `aws_apigatewayv2_route.api` at the first and `.health` at the second.
- Comment both with what is converted and why the conversion is here rather than
  in the service, citing DR-0024 and DR-0025.

`just tf-validate` is as far as this can be checked before an apply, which is
DR-0023's arrangement rather than a shortcoming.

### Documentation

Draft and confirm before the work counts as complete — `docs/README.md` requires a
human to confirm a Design Document overwrite.

- **DR-0025**, what produces the `AuthContext` in the deployment. It has to record
  the gap in DR-0024, since a reader arriving there will otherwise conclude that
  `crates/devgateway` converts in production; why `overwrite:` rather than
  `append:` is what carries DR-0024's security argument into the new arrangement;
  why the second header exists; and the alternatives rejected — a conversion
  process inside the Lambda image, and an AWS-specific module kept at
  `crates/server`'s outer edge.
- **`backend.md`** — remove the dated note at the top; the crate now matches the
  document. State the two headers. Correct the Depends-on line, which credits
  `serde_json` to the `AuthContext`; it is now used only for request and response
  bodies.
- **`deployment.md`** — the API's runtime shape and "the edge is verified here,
  not locally" both describe `x-amzn-request-context` reaching the service. The
  routes table gains the mapping and the split integration.
- **`workspace.md`** — what the adapter hands the service. Its own dated note
  stays: the reduction is still outstanding.
- **`index.md`** — the DR-0025 row.
- **The Work Log** — first action, before any code: append a Progress entry
  recording that steps 1 and 6 are superseded and why. The skill's standing
  instruction is that a wrong turn is marked, not edited away, and the reason the
  wire format changed is the useful part.

## Verification

- `just test` — the five `identity` cases and the rewritten `devgateway` tests,
  including both forgery tests.
- `just check` and `just lint`, both targets, warnings denied.
- `just dev-api` with `curl`, four requests: no headers (development owner), the
  edge header naming `alice`, the same naming `bob`, and the edge header with an
  empty subject (`401`). Creating an action type as `alice` and listing as `bob`
  is the isolation check, and it now needs neither a token nor the adapter.
- `just dev-api` and `just dev-gateway` together with a real token from
  `just dev-web-auth`, confirming the adapter's headers reach the service and name
  the token's subject.
- `just tf-validate` for the `api` layer.
- The deployed mapping, by `just tf-apply api` and a real request — the one check
  that cannot happen locally. Confirm a `/api` call is attributed to the token's
  `sub` and that `GET /health` still answers `ok`.

## Out of scope

- Reducing `crates/devgateway` — `edge.rs`, the route table, the preflight, the
  three modes, and `local.api_methods`'s hand-kept copy. That is the second
  follow-on log the policy deferred, and this plan leaves all of it standing.
- `crates/app`. DR-0024 is explicit that the SPA does not change.
- The two other open Work Logs, neither of which is reopened.
