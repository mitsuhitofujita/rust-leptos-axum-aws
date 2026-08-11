# Plan — a local stand-in for the API's edge

Carries out `docs/work/2026-08-11-local-api-edge.md`, the second of the four
phases in that log's Request section. Phase one (DynamoDB Local, DR-0020) is
already retired; phases three and four extend what this builds.

## Context

Everything between the browser and `crates/server` in a deployed request lives in
`infra/api/apigateway.tf` and in AWS. Five behaviours are decided there and are
unobservable on a developer's machine:

| Behaviour | Decided by |
| --- | --- |
| Only the methods in `local.api_methods` exist under `/api`; anything else is a 404 | `apigateway.tf` |
| `OPTIONS` is answered by the HTTP API itself, ahead of the authorizer | DR-0009 |
| A request without a valid token is refused 401 before the function is invoked | DR-0010 |
| The authorizer's claims arrive as the `x-amzn-request-context` header | DR-0017 |
| Claims are a map of strings, whatever the token held | payload format 2.0 |

The last one is a silent failure today. `crates/server/src/identity.rs:85`
deserialises `HashMap<String, String>`, so a single non-string claim fails the
decode of the whole context, `subject()` returns `None`, and the request is
attributed to `DEVELOPMENT_OWNER` instead of being refused. Nothing anywhere
observes this.

The outcome is a development-only reverse proxy that plays API Gateway and the
Lambda Web Adapter in front of the unmodified service, so those five behaviours
can be exercised locally, and phases three and four have something to build on.

## Decisions taken before writing

**The proxy target moves out of `Trunk.toml` into the recipes.** The Work Log's
plan step 6 said to point `Trunk.toml` at :3001, which would make `just dev-web`
require the stand-in — contradicting the log's own assumption that the default
two-terminal path is untouched. trunk 0.21.14 *appends* `--proxy-backend` to the
`[[proxy]]` entries rather than overriding them (`src/cmd/serve.rs:168`), so an
env-var override is not available and two entries for `/api` would collide.
Confirmed with the user: `[[proxy]]` is dropped from `Trunk.toml` and each `just`
recipe names its own backend. Step 6 is superseded and recorded as such in the
log.

**The forwarding leg is `hyper` + `hyper-util`, not `reqwest`.** `hyper` 1.11,
`hyper-util` 0.1.20, `http-body-util` 0.1.4 and `bytes` are already in
`Cargo.lock` by way of `axum`, so this adds feature flags and no packages. It is
also plain HTTP to loopback, so no TLS and therefore no OpenSSL — which is what
the log's step 1 was guarding against. Phase three's JWKS fetch will need TLS;
`hyper-rustls` 0.27 is already in the lock for it.

## Work

### 1. Record the clarification in the Work Log

Append a `### Clarifications` heading to the Request section of
`docs/work/2026-08-11-local-api-edge.md` stating the proxy-wiring answer, and
mark plan step 6 superseded with the replacement below it. Do not edit step 6's
text.

### 2. `crates/devgateway`

A binary crate beside `crates/icongen`, which is the precedent `workspace.md`
already describes for a development-only workspace member. It depends on
`shared` for nothing and must not depend on `crates/server` — DR-0017 rests on
the service being an ordinary axum binary, and compiling the stand-in into it
would destroy the property the stand-in exists to check.

New `[workspace.dependencies]` entries: `hyper` (features `client`, `http1`),
`hyper-util` (features `client-legacy`, `http1`, `tokio`), `http-body-util`,
`bytes`. `axum`, `tokio`, `serde`, `serde_json` and `base64` are already
declared. `tower` (feature `util`) as a dev-dependency for `oneshot` in tests.

| Module | Role |
| --- | --- |
| `src/main.rs` | Read the configuration, bind, serve, and print which mode is running |
| `src/config.rs` | The environment it reads, and every default |
| `src/edge.rs` | The route table, the CORS answer, and the decision function |
| `src/authorizer.rs` | The two modes: what makes a request authorized and what its claims are |
| `src/context.rs` | Building the payload-2.0 request context, claims stringified |
| `src/proxy.rs` | The forwarding leg |

Configuration, all with defaults so the binary runs with nothing set:

| Variable | Default | Meaning |
| --- | --- | --- |
| `DEVGATEWAY_ADDRESS` | `127.0.0.1:3001` | Where the stand-in listens |
| `DEVGATEWAY_UPSTREAM` | `http://127.0.0.1:3000` | The unmodified service |
| `DEVGATEWAY_MODE` | `local` | `local` or `passthrough` |
| `DEVGATEWAY_ALLOW_ORIGIN` | `http://localhost:8080` | Mirrors `cors_configuration`'s `allow_origins` |
| `DEVGATEWAY_SUBJECT` | `local-subject` | The subject used when the bearer value is not a decodable JWT |

`DEVGATEWAY_SUBJECT` deliberately does not default to `development`: a value
distinct from `identity.rs`'s `DEVELOPMENT_OWNER` is what tells a header that
arrived apart from one that was never there.

### 3. The route table — `src/edge.rs`

Mirrors `local.api_methods` in `infra/api/apigateway.tf:6`, which is `["GET",
"POST"]`, plus the probe:

```
GET|POST /api/{proxy+}   authorize, then forward
GET      /health         forward, no authorizer
OPTIONS  (anything)      answered here, never forwarded, never authorized
anything else            404 {"message":"Not Found"}
```

An unmatched route is a **404, not a 405**. A 405 is what a naive stand-in built
on an axum router would produce, and it would hide exactly the mismatch this
exists to expose: `DELETE /api/action-types` reaching a service that has no
`DELETE` handler looks the same either way, but the deployed API never gets that
far. `/api` and `/api/` are also 404 — `{proxy+}` requires a segment after the
prefix.

`OPTIONS` is answered here because no `OPTIONS` route exists, which is the same
reason the HTTP API answers it (DR-0009, and `apigateway.tf:50`). 204 with
`access-control-allow-origin`, `-methods`, `-headers` and `-max-age` mirroring
`cors_configuration` when the `Origin` is allowed, and without them when it is
not. The allow-origin header is added to forwarded responses too, as an HTTP API
adds it.

The decision is a pure function over `http::request::Parts` returning either a
response to send or the header changes to forward with, so every case above is a
unit test with no upstream and no socket.

### 4. The two behaviours that must not be approximated

**The inbound `x-amzn-request-context` is discarded unconditionally** in `local`
mode, on every path, before anything else looks at the request — along with
`x-amzn-lambda-context`. Production's safety argument is that API Gateway
overwrites the header on every request (DR-0017, and the second constraint in
`backend.md`). A rig that let a client's copy through would be a mirror in which
the header is forgeable and would teach the opposite of what is true.

`passthrough` mode forwards untouched, including that header, because it is
exactly today's `just dev-api` and is the absence of a mirror rather than a
second one. The recipe's comment says so.

**Claims are stringified** the way API Gateway stringifies them — `src/context.rs`:

| JSON claim | Becomes |
| --- | --- |
| `"abc"` | `abc` |
| `1754870400` | `1754870400` |
| `true` | `true` |
| `["a","b"]` | `[a b]` |
| anything else | its JSON rendering |

The array form is the one worth pinning: `cognito:groups` arrives as a list and
API Gateway renders it bracketed and space-separated. Every value being a string
is what makes `identity.rs`'s `HashMap<String, String>` correct, and this is the
first thing anywhere that demonstrates it.

`/health` gets a context with **no `authorizer` member at all**, which is the
shape `identity.rs`'s third test already covers and the shape the deployed
unauthenticated route really produces.

### 5. The authorizer — `src/authorizer.rs`

`local` mode, on `/api` only:

- No `Authorization` header → `401 {"message":"Unauthorized"}`, and the service
  is never reached.
- A bearer value that decodes as a JWT → its payload is the claims. Decoded, not
  verified: `base64` URL-safe-no-pad over the middle segment, parsed as a JSON
  object. Verification is phase three.
- A bearer value that is not a JWT → claims are `{"sub": DEVGATEWAY_SUBJECT}`,
  so `curl -H 'Authorization: Bearer alice'` is a usable caller.
- A JWT with no `sub` is forwarded as-is and logged. The service will attribute
  it to `DEVELOPMENT_OWNER`, which is what would happen on the Lambda too.

### 6. Ports and recipes

The stand-in on 3001, the service unchanged on 3000 —
`crates/server/src/main.rs:30` binds it as a constant and stays untouched.

`Trunk.toml` loses its `[[proxy]]` block; `[build]` and `[serve]` stay. The
`justfile` gains the backend on each recipe:

| Recipe | Change |
| --- | --- |
| `dev-web` | `trunk serve --proxy-backend http://127.0.0.1:3000/api` |
| `dev-web-auth` | the same backend, alongside the two `COGNITO_*` values |
| `dev-web-gateway` | new — the same dev server proxying to :3001 instead |
| `dev-gateway` | new — `cargo run -p devgateway` |

Recipe comments follow `workspace.md`'s rule: explanation, blank line, then the
one-line summary `just --list` shows. The `dev-gateway` comment is where the two
modes are explained, and where the fact that the rig is browser-usable only with
`dev-web-auth` is written down — a `dev-web` bundle sends no `Authorization`
header (DR-0008), so behind the stand-in every `/api` call is 401. That is not a
defect: it is the local reproduction of `deployment.md`'s constraint that a
bundle built without the two Cognito variables cannot call the API.

The default remains two terminals and no configuration. Three are needed only
when the rig is in use.

### 7. Tests — `cargo test -p devgateway`

One test per property, each naming what it pins:

1. `DELETE /api/action-types` → 404 with `{"message":"Not Found"}`, not 405.
2. `GET /nope`, `GET /api`, `GET /api/` → 404.
3. `GET /api/action-types` with no `Authorization` → 401, not forwarded.
4. `GET /health` with no `Authorization` → forwarded, context carries no
   `authorizer`.
5. A forged `x-amzn-request-context` with no token → 401.
6. A forged `x-amzn-request-context` with a token → forwarded with our context,
   subject from the token, the forgery gone.
7. Numeric, boolean and array claims → all strings; the array is `[a b]`; the
   whole context deserialises into `HashMap<String, String>`.
8. `OPTIONS /api/action-types` with an allowed `Origin` → 204 with the mirrored
   headers, no token required.
9. A non-JWT bearer value → `DEVGATEWAY_SUBJECT`.
10. `passthrough` mode → nothing added, nothing removed.

Test 7 asserts against `serde_json::from_str::<HashMap<String, String>>` rather
than against strings by eye, so it fails for the same reason `identity.rs` would.

End-to-end tests over running processes are phase four and are not written here.

### 8. Documents to draft, for confirmation before the work is called complete

Design Documents are overwritten by nature, so these are drafted and confirmed
rather than committed silently (`docs/README.md`, Ownership).

- **`docs/decisions/DR-0021-the-deployed-edge-is-reproduced-outside-the-service.md`** —
  new, append-only, no confirmation needed. Context: the five unobservable
  behaviours and the silent claim-decode failure. Decision: a development-only
  reverse proxy outside the service. Alternatives: a mode inside `crates/server`
  (destroys what DR-0017 asserts), SAM/LocalStack (a second toolchain and no
  Python in this container), `cargo lambda` (emulates the runtime API, not the
  authorizer or the route table). Consequences: the route table and the CORS
  configuration are copies of `apigateway.tf` kept in step by hand — the same
  arrangement `justfile`'s `dynamo_table` and `project` already have.
- **`workspace.md`** — `devgateway` in the layout, the four recipe rows, the
  proxy target moving into the recipes, and constraints: the stand-in ships
  nothing; the route table is a hand-maintained copy of `local.api_methods`; a
  bare `trunk serve` outside `just` no longer proxies.
- **`deployment.md`** — under "The API's runtime shape", that the edge's
  behaviour is mirrored locally and where the mirror is, and that a change to
  `local.api_methods` now has a second place to follow it.
- **`frontend.md`** — `Trunk.toml` no longer holds the `/api` proxy; the
  constraint bullet naming 127.0.0.1:3000 gains the recipes and the :3001
  variant.
- **`docs/design/index.md`** — the DR-0021 row.
- The Work Log's Progress, Verification and Retirement sections, dated.

## Verification

Automated, and expected to pass before anything manual:

```sh
just fmt-check && just check && just lint && just test
```

Manual, with `just dev-api` in one terminal and `just dev-gateway` in another:

```sh
# 404, not 405, and the service is never reached
curl -i -X DELETE localhost:3001/api/action-types

# 401 before the service is invoked
curl -i localhost:3001/api/action-types

# a forged context does not become the caller
curl -i -H 'x-amzn-request-context: {"authorizer":{"jwt":{"claims":{"sub":"attacker"}}}}' \
     localhost:3001/api/action-types            # expect 401

# a bearer value that is not a JWT is a usable caller
curl -i -H 'Authorization: Bearer alice' localhost:3001/api/action-types

# the probe, unauthenticated, with a context carrying no authorizer
curl -i localhost:3001/health

# preflight answered without a token
curl -i -X OPTIONS -H 'Origin: http://localhost:8080' \
     -H 'Access-Control-Request-Method: POST' localhost:3001/api/action-types
```

Then that the isolation actually holds: `POST` an action type with
`Authorization: Bearer alice`, another with `Bearer bob`, and check each `GET`
sees only its own — the whole reason `identity::Owner` exists, and something
nothing has been able to check locally until now.

Finally, that the default path is undisturbed — `just dev-web` with only
`just dev-api` running still serves the SPA and its `/api` calls — and that
`just dev-web-gateway` serves it through the stand-in, where the API calls are
401 until phase three or `just dev-web-auth` supplies a token.
