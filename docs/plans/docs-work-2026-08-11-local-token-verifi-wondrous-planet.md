# Plan — Verifying real Cognito tokens locally

Executes `docs/work/2026-08-11-local-token-verification.md`, the third of the four
phases of the local-verification work.

## Context

`crates/devgateway` (DR-0021) already plays API Gateway and the Lambda Web Adapter
in front of the unmodified service, but its authorizer *decodes* a token without
verifying it. That is enough to exercise everything downstream — the request
context, the subject, `identity::Owner`'s isolation — and says nothing about
whether `aws_apigatewayv2_authorizer.cognito` would have accepted the same token.

The gap is not the service's problem; the service is indifferent. It belongs to
`infra/api/apigateway.tf`, whose `jwt_configuration` has a small number of ways to
be subtly wrong, every one of which surfaces only as a 401 after an apply, with
nothing distinguishing it from a broken sign-in:

- `audience` holds the app client id, which a Cognito **access** token carries as
  `client_id` and an **id** token as `aud`. A stand-in checking only `aud` would
  accept what API Gateway rejects, and the reverse.
- `issuer` must be the pool's issuer URL exactly.
- The SPA must be sending the access token, not the id token
  (`crates/app/src/api.rs:71` reads `auth::access_token`).

The intended outcome is a third `devgateway` mode that does locally what the
deployed authorizer does, so that those three can be checked before an apply.

Out of scope, per the Work Log: obtaining tokens (`just dev-web-auth` already
does), being an enforcement boundary, scopes (the deployed authorizer declares
none), and any change to `crates/server` or `crates/app`.

## Decisions taken before starting

Confirmed with the user:

1. **The JWKS is fetched in-process, once, at startup.** `hyper-rustls` 0.27,
   `rustls` 0.23 and `rustls-native-certs` are already in `Cargo.lock` beneath
   `aws-sdk-dynamodb`, so this adds no package and no system package — the
   property DR-0021 records for `hyper-util` still holds. `/etc/ssl/certs/ca-certificates.crt`
   is present in the image. Fetching at startup rather than lazily keeps
   `edge::decide` synchronous and gives "cache for the process's lifetime"
   literally.
2. **Signature verification uses `aws-lc-rs` directly.**
   `RsaPublicKeyComponents { n, e }.verify(&RSA_PKCS1_2048_8192_SHA256, …)` takes
   the JWKS `n`/`e` in exactly the form they arrive in, needs no ASN.1 parsing,
   and is already compiled in this workspace as `rustls` 0.23's default backend.
   `jsonwebtoken` was rejected: it adds `pem`, `simple_asn1`, `num-bigint`,
   `num-traits` and a second crypto backend, and its audience validation has to be
   switched off anyway because API Gateway's rule is not its rule.
3. **A Decision Record is written — DR-0022.** DR-0021 decided that the edge is
   reproduced outside the service, not that real tokens are verified against the
   real pool.

## Implementation

### 1. `crates/devgateway/src/jwks.rs` — new

The key set, fetched once and held for the process's lifetime.

- `pub struct Keys(HashMap<String, Rsa>)` where `Rsa { n: Vec<u8>, e: Vec<u8> }`,
  both base64url-decoded at parse time.
- `Keys::parse(&[u8]) -> Result<Keys, String>` over the JWKS document. Skip
  entries that are not `{"kty":"RSA","alg":"RS256"}`; an empty result is an error.
  Keeping parsing separate from fetching is what makes the tests below need no
  network.
- `pub async fn fetch(issuer: &str) -> Result<Keys, String>`, over
  `{issuer}/.well-known/jwks.json` — the URL Cognito publishes and API Gateway
  reads. Build the connector with
  `HttpsConnectorBuilder::new().with_native_roots()?.https_only().enable_http1()`,
  then `Client::builder(TokioExecutor::new()).build(connector)`. Collect the body
  with `axum::body::to_bytes` (axum 0.8 has it) under a small cap, so
  `http-body-util` need not be declared.
- `pub fn verify(&self, kid: &str, signing_input: &[u8], signature: &[u8]) -> Result<(), String>`.
  An unknown `kid` is an error here, not a fetch — the Work Log is explicit that a
  `kid` miss is a 401 and not a refetch loop.

Unit tests: `Keys::parse` on a two-key document; an unknown `kid`; a document with
a non-RSA entry.

### 2. `crates/devgateway/src/config.rs`

- `Mode` gains `Cognito`. `mode()` keeps refusing an unrecognised value.
- New `pub struct Verification { pub issuer: String, pub audience: String }`, read
  from `DEVGATEWAY_ISSUER` and `DEVGATEWAY_AUDIENCE`. Both are **required** when
  the mode is `cognito` and an unset one is a startup error naming the recipe —
  the same "refused rather than defaulted" treatment `mode()` already gives, and
  for a stronger reason: a defaulted issuer would silently verify against the
  wrong pool.
- `Config` gains `pub verification: Option<Verification>`, `Some` iff the mode is
  `Cognito`. `Config::for_test` grows a variant (or an argument) supplying one.

### 3. `crates/devgateway/src/authorizer.rs`

Keep `bearer` and `claims` as they are. Add the verifying path beside the
decoding one:

- `pub struct Verifier { pub verification: Verification, pub keys: Keys }`, held
  by `main` and reached from `decide`.
- `pub fn authorize(parts, verifier: Option<&Verifier>) -> Authorization` —
  `None` is today's behaviour unchanged; `Some` verifies.
- `fn verified(token, verifier, now: u64) -> Result<Map<String, Value>, String>`,
  with the clock injected so the tests are not time-dependent:
  1. Split into three segments; decode header and payload.
  2. `alg` must be `RS256`; read `kid`.
  3. `keys.verify(kid, b"{header}.{payload}", signature)`.
  4. `iss` equals `verification.issuer` exactly.
  5. `exp` is in the future (and `nbf`, if present, in the past).
  6. **Audience, the way API Gateway checks it**: satisfied if `client_id` equals
     the configured id (access token) **or** `aud` contains it, as a string or as
     an array (id token). Either satisfies, neither is a refusal.
  Each step's `Err` is a sentence naming what was expected and what arrived, and
  the audience step's mentions `token_use` so that "you are sending the id token"
  reads as itself.
- Every `Err` becomes `Authorization::Refused` after being printed with a
  `devgateway:` prefix. The reason is logged and never returned — the deployed
  authorizer answers `401 {"message":"Unauthorized"}` and nothing else, which
  `edge::answer` already produces.
- A bearer value that is not a JWT is refused in this mode. `Bearer alice` is a
  `local`-mode affordance and cannot survive verification; the mode announcement
  and the docs say so rather than leaving it to be discovered.

### 4. `crates/devgateway/src/edge.rs`

`decide` takes the verifier alongside the config (or reads it from a struct passed
in) and hands it to `authorizer::authorize`. `Mode::Cognito` follows exactly the
same path as `Mode::Local` — same route table, same preflight, same discarding of
`x-amzn-request-context`, same request context — differing only in the
authorizer's verdict. Nothing else in the file changes; `Mode::Passthrough` still
short-circuits first.

### 5. `crates/devgateway/src/main.rs`

`main` fetches the key set before binding, when the mode is `Cognito`, and exits
with the reason if it cannot. `announce` gains its line: the issuer, the audience,
how many keys were loaded, and that `Bearer alice` no longer works here.

### 6. `crates/devgateway/Cargo.toml` and root `Cargo.toml`

Add to `[workspace.dependencies]`, each with the comment the file's existing
entries set the style for — that both are already in `Cargo.lock` beneath the AWS
SDK, so this adds features and no packages:

```toml
hyper-rustls = "0.27.9"
aws-lc-rs = "1.18.0"
```

`crates/devgateway` declares `hyper-rustls` with `rustls-native-certs`,
`http1` and `ring`-free defaults as needed, and `aws-lc-rs`.

### 7. `justfile`

`dev-gateway-cognito`, beside `dev-gateway`, resolving both values from SSM
through the existing `_ssm` recipe — the same shape `dev-web-auth` already uses,
so it needs AWS credentials for SSM and network for the JWKS, but no more:

```just
dev-gateway-cognito:
    #!/usr/bin/env bash
    set -euo pipefail

    DEVGATEWAY_MODE=cognito \
    DEVGATEWAY_ISSUER="$(just _ssm identity/user_pool_issuer)" \
    DEVGATEWAY_AUDIENCE="$(just _ssm identity/app_client_id)" \
        cargo run -p devgateway
```

Its comment obeys the `just --list` rule `workspace.md` records: explanation,
blank line, one-line summary.

### 8. Tests

A fixed 2048-bit RSA key generated once and committed under
`crates/devgateway/tests/` (or `src/testdata/`): the PKCS#8 DER private key via
`include_bytes!`, and the matching JWKS as a `&str`. Minted with what the image
has —

```
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -outform DER -out key.der
openssl rsa -in key.der -inform DER -pubout -outform DER | \
  openssl rsa -pubin -inform DER -noout -modulus | sed 's/^Modulus=//' | \
  basenc --base16 -d | basenc --base64url -w0
```

with `e` = `AQAB`. Tests sign with `aws_lc_rs::signature::RsaKeyPair::from_pkcs8`,
so any claim set is expressible, and pass a fixed `now`.

Cases, one test each:

| Case | What it pins |
| --- | --- |
| A well-formed access token (`client_id` = audience) is allowed | the mode works at all |
| An id token (`aud` = audience, no `client_id`) is allowed | the audience rule's other half |
| A token whose signature has one byte changed is refused | the thing `local` mode cannot do |
| An expired token is refused | `exp` |
| A token from another app client is refused | the `audience` misconfiguration |
| A token from another issuer is refused | the `issuer` misconfiguration |
| A `kid` outside the key set is refused, without a refetch | the Work Log's explicit requirement |
| `Bearer alice` is refused in `cognito` mode | the trade-off against `local` mode |
| The same token is *allowed* in `local` mode | the two modes differ in exactly one thing |

Plus an `edge.rs` test that a refusal in `cognito` mode is
`401 {"message":"Unauthorized"}` with no reason in the body.

### 9. Documents

Drafted for confirmation, per `docs/README.md`'s ownership rule — a human
confirms a Design Document overwrite.

- **`docs/decisions/DR-0022-*.md`** — real tokens are verified locally against the
  pool's JWKS, in the stand-in and not in the service. Context: the three ways
  `jwt_configuration` can be wrong and the fact that each is indistinguishable
  from a broken sign-in after an apply. Decision: the `cognito` mode, verified
  in-process, refusing with the deployed body and logging the reason.
  Alternatives: `jsonwebtoken`; a JWKS fetched by the recipe with `curl`;
  verifying in `crates/server` (which DR-0017 forbids and this does not touch).
  Consequences: the mode needs credentials, network and a real pool, so it is not
  the default and not part of `just test`; `Bearer alice` does not work in it; the
  audience rule is a hand-maintained copy of API Gateway's behaviour, as the route
  table already is.
- **`docs/design/deployment.md`** — extend the "The edge is reproduced locally"
  paragraph (around line 186) and add a constraint stating that the authorizer's
  `issuer` and `audience` are checkable before an apply, by `just
  dev-gateway-cognito`, and that `audience` is matched against `client_id` for an
  access token and `aud` for an id token.
- **`docs/design/workspace.md`** — `dev-gateway-cognito` in the recipe table, a
  sentence in the `dev-gateway` paragraph (around line 108) about the third mode
  and what it costs, and an amendment to the `crates/devgateway` paragraph (around
  line 42), whose current text says `hyper-util` is its only third-party
  dependency beyond the service's and that no TLS stack came with it — both now
  need restating.
- **`docs/design/index.md`** — the DR-0022 row.

### 10. Work Log

Append to `Progress` as the work lands, record `Verification` results, and tick
the `Retirement` checklist. The log's own note that the DR question is "to be
judged when it is built" is answered by DR-0022 rather than deleted — the Request
and Plan sections are append-oriented and not rewritten.

## Verification

Ordered so that everything needing no AWS runs first.

1. `just fmt-check`, `just check`, `just lint`, `just test` — the unit tests
   above, no network, no credentials.
2. `just dev-gateway` still starts and behaves as before with nothing set; a
   `curl -H 'Authorization: Bearer alice' localhost:3001/api/action-types` against
   a running `just dev-api` still works. The default is untouched.
3. `DEVGATEWAY_MODE=cognito cargo run -p devgateway` with no issuer set fails at
   startup with a message naming the recipe.
4. With AWS credentials, in three terminals — `just dev-api`,
   `just dev-gateway-cognito`, `just dev-web-auth`:
   - the announcement names the issuer, the audience and the key count;
   - a token from the browser's `localStorage` (`auth.access_token`), passed as
     `curl -H "Authorization: Bearer $TOKEN" localhost:3001/api/action-types`, is
     allowed and the subject reaches the service;
   - the same token with one character of its signature changed is a 401, with
     the reason on the gateway's terminal and not in the response;
   - the id token in place of the access token is **allowed** — both satisfy the
     one configured app client — and the log line says which arrived, which is
     the check that the SPA sending the wrong one would be visible;
   - `DEVGATEWAY_AUDIENCE=wrong just dev-gateway-cognito` refuses the same good
     token, which is the misconfiguration this phase exists to catch, reproduced
     deliberately.
5. Browse the SPA at `http://localhost:8080` through `just dev-web-gateway`
   pointed at the same gateway, signed in, and confirm the dashboard loads —
   an end-to-end pass with a verified token.

Step 4's last two items are the return on the whole phase; if only one check is
run by hand, it is that one.
