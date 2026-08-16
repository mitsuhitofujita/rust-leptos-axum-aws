# Backend

Updated: 2026-08-16

## Purpose

The axum service in `crates/server`: what it exposes, who it thinks is calling,
and where it keeps what it is given.

It is an ordinary axum binary. Nothing in it is written for Lambda — the Lambda
Web Adapter turns an invocation into an HTTP request on `127.0.0.1:3000`, and
`just dev-api` runs the same binary with nothing in front of it (DR-0001).

## Structure

| File | Role |
| --- | --- |
| `src/main.rs` | The router, the shared state, and which store and auth arrangement the process is using |
| `src/identity.rs` | Who the caller is |
| `src/cognito.rs` | Verifying a Cognito token's signature, issuer, expiry and audience |
| `src/jwks.rs` | The pool's signing keys: fetch, parse, verify by `kid` |
| `src/store.rs` | Reading and writing the table |
| `src/action_types.rs` | `/api/action-types`, and what may be stored |
| `src/dashboard.rs` | `/api/dashboard`, still answering from fixed values |
| `src/testkey.rs` | A committed RSA fixture the identity/cognito/jwks tests sign with — `#[cfg(test)]` only |

`AppState { store: Arc<Store>, auth: Arc<identity::Auth> }` is the router's
state, built once at startup. Both the SDK client and the pool's key set are
expensive to construct and cheap to share, and which store and which auth
arrangement the process is using cannot change while it runs.

**Identity.** `identity::Owner` is an axum extractor, and it is the whole of
user isolation. A handler asks for the owner and cannot ask for anything else:
no path, query or body parameter names one, which is what stops a handler from
serving a partition its caller does not own — and, since DR-0028, no route
grants this for free: a handler that does not name `Owner` is reachable by
anyone with no token, so every handler under `/api` names it, `dashboard()`
included even though it does nothing with the value yet.

**Two arrangements, chosen once at startup by `identity::Auth::from_environment`**,
the same shape `store::Store` already uses for which store to run:

| Arrangement | Selected when | Produces the owner from |
| --- | --- | --- |
| `Auth::Cognito` | `COGNITO_ISSUER` and `COGNITO_AUDIENCE` both set | A real Cognito token, verified against the pool |
| `Auth::Mock` | Both unset — `just dev-api`'s default | Two headers, set by hand |

`Auth::Cognito` fetches the pool's key set once, before the listener binds — a
pool that cannot be reached is a reason to stop, with the reason on screen,
not to accept connections and refuse every one of them for a cause nobody can
see. It then verifies RS256 against the key the token's `kid` names, then
`iss`, `exp`, and the audience — satisfied by `client_id` for an access token
or `aud` for an id token, because a Cognito access token carries the app
client id one way and an id token the other, and both are accepted (DR-0028).
Every failure is refused: no token, a bad signature, a wrong issuer or
audience, an expired token, a token with no usable `sub`. There is no
development-owner fallback in this arrangement at all.

`Auth::Mock` reads two headers:

| Header | Carries |
| --- | --- |
| `x-auth-subject` | The subject a caller asserts by hand |
| `x-auth-edge` | Nothing. Its presence is the assertion that a caller means to name someone |

Three cases, and the difference between the last two is the point:

| What arrives | Who the owner is |
| --- | --- |
| `x-auth-edge`, and a non-empty `x-auth-subject` | That subject |
| Neither header | A constant development owner — DR-0018 |
| `x-auth-edge` with `x-auth-subject` absent or empty | Nobody. The request is refused with `401` |

Absent means development, but **only under `Auth::Mock`**: nothing asserted
anything, so there is nothing to misread, and that is what makes `just
dev-api` a working application with no configuration. Under `Auth::Cognito`
this fallback does not exist — a missing token is refused exactly like a bad
one. Being a particular caller under `just dev-api` is both headers sent by
hand — `x-auth-edge: curl` and `x-auth-subject: alice`. Two callers are one
`curl` flag apart, with no token and no AWS credentials, which is what makes
owner isolation checkable at all locally.

**The safety argument is which variant is running, not anything in front of
the service.** `Auth::Mock` is the only arrangement that ever reads the two
headers, and it is only active when `COGNITO_ISSUER`/`COGNITO_AUDIENCE` are
unset — which the deployed Lambda never has, since Terraform always sets
both, mirroring `TABLE_NAME`. The deployed function is therefore always
`Auth::Cognito`, which never reads either header at all, whatever a caller
sends (DR-0028).

**The store.** `store::Store` is an enum with two variants, chosen from the
environment at startup: `TABLE_NAME` set selects DynamoDB, unset selects an
in-memory map. Terraform sets it on the Lambda and nothing sets it locally, so
the deployed service and the development server differ by configuration rather
than by code (DR-0018). An enum rather than a trait object because there are
exactly two and the choice is settled before the first request.

The DynamoDB variant is run locally by `just dev-api-dynamo`, against the
DynamoDB Local pinned in the devcontainer image (DR-0020). That recipe sets
`TABLE_NAME` and points the SDK at `http://localhost:8000` with fake
credentials; the binary is the same one, taking the same branch it takes on the
Lambda, which is what makes it a check of the deployed path rather than of
something resembling it.

Keys, attributes and queries are `persistence.md`'s, not this document's. What
belongs here is that the identifier is minted by the service — a ULID, so it
sorts by creation time and needs no coordination — and that what leaves the
service is the bare ULID, never the `TYPE#`-prefixed key.

**Validation.** `action_types::validate` is where a request stops being
whatever was sent. Names and units are trimmed and then required to be
non-empty, so a field of spaces is refused where the browser's `required`
attribute would accept it; both are length-limited, counted in characters. The
icon must be a name in `shared::icon_names`, the same catalog the picker was
generated from — the picker is the only control surface that offers one, but a
request need not have come from it (DR-0014, DR-0019).

**Failures.** `action_types::Failure` separates the two kinds. A rejected
request answers `400` with the reason in plain words, because that reason is
what the screen shows; a store that did not answer is logged and answered `500`
with a sentence the visitor can do nothing with, because there is nothing they
could do.

## Interfaces

**Exposes**

| Route | Answers |
| --- | --- |
| `GET /health` | `ok`. Deliberately unauthenticated, because a probe carries no token |
| `GET /api/dashboard` | `shared::Dashboard`, from hardcoded values |
| `GET /api/action-types` | `shared::ActionType[]`, oldest first |
| `POST /api/action-types` | `201` and the stored `shared::ActionType`, from a `shared::NewActionType` |

No CORS layer. Development is single-origin through the trunk proxy, and
production is answered by the HTTP API rather than here (DR-0009). A new method
under `/api` needs `local.api_methods` in `infra/api/apigateway.tf` to name it;
a new path does not.

**Depends on** `axum` and `tokio`, `aws-config` and `aws-sdk-dynamodb`, `ulid`,
`time` for one formatted instant, and `shared`. Also `hyper-util` and
`hyper-rustls` for the JWKS fetch, `aws-lc-rs` for both the TLS leg and the
RS256 signature check, and `serde`/`serde_json`/`base64` for the key set and
the token payload — all five already resolved in `Cargo.lock` beneath
`aws-sdk-dynamodb`, so declaring them here adds dependency edges and no
packages (DR-0028).

**Reads** `TABLE_NAME`, `COGNITO_ISSUER` and `COGNITO_AUDIENCE` from the
environment, and nothing else besides what the SDK reads underneath — the
region, the credentials, and the `AWS_ENDPOINT_URL_DYNAMODB` that `just
dev-api-dynamo` redirects the client with. None of the SDK's variables is
named in this crate, which is why running against a local DynamoDB cost it no
code (DR-0020).

## Constraints

- **The owner comes from the `AuthContext` and from nowhere else.** The IAM
  policy cannot express user isolation — the function serves every user, so its
  permissions cover every partition — so a handler that took an owner from a
  request parameter would defeat it entirely — DR-0024.
- **Every handler under `/api` must name `Owner`, even one that does not yet
  use the value.** Gating moved from the route table to the handler's own
  signature — DR-0028. A handler that forgets is reachable by anyone with no
  token; `dashboard()` names it unused for exactly this reason.
- **`COGNITO_AUDIENCE` is a hand-maintained copy of what Cognito puts in a
  token's `client_id`/`aud`, and `COGNITO_ISSUER` of the pool's issuer URL.**
  Both come from SSM, resolved the same way `deploy-web`'s Cognito variables
  are; a wrong one refuses every real token with a `401` that looks identical
  to a broken sign-in — `just dev-api-cognito` exists to make that checkable
  before a deploy rather than after one (DR-0028).
- **Exactly one of `COGNITO_ISSUER`/`COGNITO_AUDIENCE` set is a startup
  error, not a fallback to `Auth::Mock`.** A half-configured environment must
  never silently downgrade to header-trusting mode — DR-0028.
- **A missing `AuthContext` under `Auth::Mock` means development; a marked one
  with no subject means rejection. Under `Auth::Cognito`, a missing token is
  refused with no fallback at all.** The first pair is unreachable in a
  deployed function, because Terraform always sets both Cognito variables,
  selecting `Auth::Cognito` — DR-0018, DR-0028.
- **`created_at` and the instant inside a record's sort key must be fixed-width
  RFC 3339 in UTC.** `store::TIMESTAMP` is the only thing enforcing it, and a
  variable-width instant fails silently — DR-0015.
- **The in-memory store is not a second design, and can still drift.** It
  answers from insertion order where DynamoDB answers a `Query` in key order,
  and those agree only because the key embeds a ULID. Anything that changes the
  key encoding changes both — DR-0018. `cargo test` reaches only the in-memory
  half; the other one is checked by running `just dev-api-dynamo` by hand, which
  nothing does automatically — DR-0020.
- **`Scan` is not granted and no access pattern needs one.** Every query is
  inside one owner's partition — `persistence.md`.
- **The dashboard is not connected to the store.** It answers from values in
  `src/dashboard.rs`. Only the body of that handler changes when it is.
