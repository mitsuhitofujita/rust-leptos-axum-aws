# Verifying real Cognito tokens locally

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

To be appended.

## Verification

To be recorded. The intended check is a real token from `just dev-web-auth`
accepted in `cognito` mode with the subject reaching the service; the same token
rejected after tampering with a byte of its signature; an expired token rejected;
and a token from a different app client rejected.

## Retirement

- [ ] Design Documents updated — `deployment.md`, `workspace.md`
- [ ] Decision Records written (DR-____), if the second phase's does not cover it
- [ ] Non-obvious knowledge preserved — that the audience lives in `client_id`
      for an access token and `aud` for an id token, and that this is the
      distinction the deployed authorizer's configuration turns on
- [ ] No durable document depends on this log
