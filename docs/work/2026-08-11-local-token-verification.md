# Verifying real Cognito tokens locally

Status: in progress — built and checked without AWS; the four checks against a
real pool remain, and the Design Document updates await confirmation
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

**This log answers the third phase: verifying real tokens.** The others are
`2026-08-11-local-dynamodb.md`, `2026-08-11-local-api-edge.md` and
`2026-08-11-end-to-end-verification.md`. This phase extends the crate the second
one creates and cannot be done before it.

## Interpretation

**What is being asked.** The second phase reads a token without verifying it,
which is enough to exercise the service but says nothing about whether the
deployed authorizer would have accepted the same token. This phase closes that
gap by doing locally what `aws_apigatewayv2_authorizer.cognito` does: fetch the
pool's JWKS, verify the signature, and check the issuer, the expiry and the
audience.

**What it is actually for.** Not the service — the service is indifferent. It is
for `infra/api/apigateway.tf`. The authorizer's configuration has a small number
of ways to be subtly wrong, and every one of them currently surfaces as a 401
after a deploy, with nothing to distinguish it from a broken sign-in:

- `audience` holds the app client id, which a Cognito **access** token carries as
  `client_id` and an **id** token as `aud`. A stand-in that only checks `aud`
  would accept what API Gateway rejects, and the reverse.
- `issuer` must be the pool's issuer URL exactly.
- The SPA must be sending the access token rather than the id token.

Checking these before an apply is the whole return on this phase.

**Out of scope.**

- Obtaining tokens. `just dev-web-auth` already does that, and this phase reuses
  it.
- Being an authorizer. Nothing enforces anything locally; this mode exists to
  make a wrong configuration visible, not to protect a development machine.
- Scopes. The deployed authorizer declares none, so there is nothing to mirror.
- Any change to `crates/server` or to the sign-in flow in `crates/app`.

**Assumptions.**

- The JWKS endpoint is public, so this mode needs network access but no AWS
  credentials. Credentials are needed only to resolve the pool's identifiers from
  SSM, which is what `just dev-web-auth` already needs them for.
- Verifying by hand here does not contradict DR-0017's refusal to verify tokens
  in `crates/server`. That refusal is about the service; this is the thing
  standing in for the component whose job verification is.

## Plan

1. **A `cognito` mode** in `crates/devgateway`, alongside `passthrough` and
   `local`. Selected by configuration, never the default.
2. **JWKS**: fetch once, cache for the process's lifetime, select the key by the
   token header's `kid`. A `kid` that is not in the set is a 401, not a fetch
   loop.
3. **Verification**: RS256 signature, `iss` against the pool issuer, `exp`, and
   the audience checked the way API Gateway checks it — `client_id` for an access
   token, `aud` for an id token, either satisfying the configured app client id.
4. **Every failure answers `401 {"message":"Unauthorized"}`**, as the deployed
   authorizer does, with the reason logged rather than returned. The reason is
   what a developer needs and what a caller must not have.
5. **`just dev-gateway-cognito`**, resolving the issuer and the app client id
   from SSM in the same shape `dev-web-auth` already uses.
6. **Documents.** Draft updates to `deployment.md` — that the authorizer's
   configuration is checkable before an apply, and how — and to `workspace.md`
   for the recipe, for confirmation. Whether this needs its own Decision Record
   is to be judged when it is built; the second phase's record may already cover
   it.

## Progress

**2026-08-11 — two dependency questions settled before writing anything.** The
plan's step 2 says "fetch the JWKS", which needs HTTPS, and `crates/devgateway`
had no TLS stack — DR-0021 records the absence of one as a property worth having.
Two ways out were weighed: fetching in-process, or having the `just` recipe
`curl` the document and pass it in.

Reading `Cargo.lock` settled it. `hyper-rustls` 0.27, `rustls` 0.23 and
`rustls-native-certs` are all already there beneath `aws-sdk-dynamodb`, and
`/etc/ssl/certs/ca-certificates.crt` is in the image. The in-process fetch
therefore costs nothing that DR-0021 was protecting, and the recipe-side
alternative would have made `cognito` mode unusable except through `just`. Fetched
in-process, at startup.

The same reading settled the signature check. `aws-lc-rs` is `rustls` 0.23's
backend and so is already compiled here, and its
`RsaPublicKeyComponents { n, e }` takes the JWKS components in the exact form they
arrive in — no ASN.1. `jsonwebtoken` would have added four packages and a second
cryptographic backend, and its audience validation would have had to be switched
off anyway, because API Gateway's rule is not its rule. Both confirmed with the
requester before starting.

**Fetching before the listener binds turned out to be the better shape**, not just
the simpler one. It makes "cache for the process's lifetime" literal, it keeps
`edge::decide` synchronous so nothing about `edge.rs` had to change beyond one
argument, and it turns an unreachable pool into a startup failure with the reason
on screen rather than a process that accepts connections and refuses every one of
them for a cause nobody can see.

**What was built.** `jwks.rs` (fetch, parse, verify-by-`kid`), `Mode::Cognito` and
a `Verification` in `config.rs`, a `Verifier` and a `verified()` path in
`authorizer.rs`, one extra argument through `edge::decide`, the eager fetch and
the announcement in `main.rs`, and `just dev-gateway-cognito`. `local` and
`passthrough` are untouched; `Local` and `Cognito` take the same path through
`edge.rs` and differ only in the authorizer's verdict.

**Two findings worth keeping.**

`openssl genpkey -outform DER` writes PKCS#1, not PKCS#8, whatever the flag
suggests, and `RsaKeyPair::from_pkcs8` rejects it with `InvalidEncoding`. The
fixture needs a second pass through `openssl pkcs8 -topk8 -nocrypt`. The recipe in
`testkey.rs` carries both commands and the reason.

Declaring `aws-lc-rs` with its default features added exactly one package to
`Cargo.lock`: `untrusted` 0.7.1, beside the 0.9 already present, pulled by
`ring-io` and `ring-sig-verify`. Those two are a compatibility surface for code
ported from `ring`, and nothing here is. With `default-features = false` the lock
gains two dependency edges and no packages at all, which is what makes the claim
in `workspace.md` literally true rather than nearly true. The only casualty is
`public_key().modulus_len()`, and `public_modulus_len()` on the key pair is the
same number.

**`Bearer alice` cannot survive this mode**, which was not in the plan and is the
one real cost. DR-0021's most useful affordance depends on the bearer value being
taken at its word. Rather than special-casing it — which would have made the mode
a liar about what it verifies — the two modes are documented as complementary,
the startup announcement says so, and a test pins it.

**A Decision Record was warranted after all.** The plan's step 6 left it open.
DR-0021 decided that the edge is reproduced outside the service; it did not decide
that real tokens are verified against the real pool, and the audience rule, the
`Bearer alice` trade-off, and the rejected alternatives have no home in a Design
Document. DR-0022.

## Verification

Everything that needs no AWS was run and passed.

- `just fmt-check`, `just check`, `just lint`, `just test` — all green. 39 tests
  in `crates/devgateway`, up from 26; the thirteen new ones cover a well-formed
  access token, an id token, an `aud` array, a tampered signature, a swapped
  subject, an expired token, another app client, another issuer, an unknown
  `kid`, `alg: none`, `Bearer alice` refused, the same token that `cognito`
  refuses being forwarded by `local`, and the whole `authorize` path end to end.
  All sign with the committed fixture key, and all but the last check against a
  fixed clock, so none of them touches the network or the date.
- The two log lines, seen under `--nocapture`, read as intended:
  `devgateway: 200 — verified access token for sub=abc-123, audience matched on
  `client_id`` and `devgateway: 401 — the token expired … seconds ago.` The first
  is the one that tells an access token from an id token, which is the diagnosis
  this phase is for.
- `DEVGATEWAY_MODE=cognito` with `DEVGATEWAY_ISSUER` unset exits 1 with
  `... but DEVGATEWAY_ISSUER is unset. `just dev-gateway-cognito` resolves it
  from SSM.` A misspelled mode is refused too, naming the three.
- **The TLS leg works against a real AWS endpoint.** Pointed at
  `https://cognito-idp.ap-northeast-1.amazonaws.com/ap-northeast-1_doesnotexist`,
  the process exits 1 with `... /.well-known/jwks.json answered 404 Not Found` —
  which is the far side answering over TLS, so the handshake, the native root
  store and the HTTPS GET are all exercised without a pool and without
  credentials.
- **The default is untouched.** `devgateway` and `server` started together with
  nothing set: the announcement is unchanged, `/api/action-types` without a token
  is 401, with `Bearer alice` is 200, and `/health` is 200.
- `Cargo.lock` gains two dependency edges under `devgateway` and no packages.

Not yet run, because it needs AWS credentials and a real pool — the four checks
this phase's plan named:

- a real token from `just dev-web-auth` accepted, with the subject reaching the
  service;
- the same token refused after one byte of its signature is changed;
- an expired token refused;
- `DEVGATEWAY_AUDIENCE` set to something else refusing the same good token, which
  is the deployed misconfiguration reproduced deliberately.

The first three are pinned by the unit tests above against the fixture key; what
the manual run adds is that the pool's real key set parses and that the real
issuer and app client id resolve from SSM into a working configuration.

## Retirement

- [x] Design Documents updated — `deployment.md`, `workspace.md`, and the
      `index.md` record table (drafted; awaiting confirmation, per `docs/README.md`)
- [x] Decision Records written — DR-0022. The second phase's did not cover it:
      DR-0021 decided that the edge is reproduced outside the service, not that
      real tokens are verified against the real pool
- [x] Non-obvious knowledge preserved — the audience living in `client_id` for an
      access token and `aud` for an id token is a `deployment.md` constraint and
      DR-0022's Context; the `Bearer alice` trade-off, the `jsonwebtoken`
      rejection and the eager fetch are DR-0022's Decision and Consequences; the
      `openssl` PKCS#1/PKCS#8 trap and the `aws-lc-rs` default-features trap are
      in `testkey.rs` and the root `Cargo.toml` beside the code they explain
- [ ] No durable document depends on this log
