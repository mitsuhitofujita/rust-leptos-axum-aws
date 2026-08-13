# DR-0021: The deployed edge is reproduced locally, outside the service

Status: superseded by DR-0023
Date: 2026-08-11

## Context

`crates/server` is written against an edge it never meets outside AWS. Between
the browser and the service in a deployed request sit an API Gateway HTTP API and
the Lambda Web Adapter, and five of their behaviours are decided in
`infra/api/apigateway.tf`:

| Behaviour | Decided by |
| --- | --- |
| Only the methods in `local.api_methods` exist under `/api`; anything else is a 404 | `apigateway.tf` |
| `OPTIONS` is answered by the HTTP API itself, ahead of the authorizer | DR-0009 |
| A request without a valid token is refused 401 before the function is invoked | DR-0010 |
| The authorizer's claims arrive as the `x-amzn-request-context` header | DR-0017 |
| Every claim is a string, whatever the token held | payload format 2.0 |

None of them could be observed on a developer's machine. `just dev-api` runs the
service with nothing in front of it, so every request is unauthenticated, is
attributed to the development owner (DR-0018), and reaches whatever handler axum
has for it.

Two consequences of that are worse than an absence of coverage.

The first is silent. `crates/server/src/identity.rs` deserialises the claims as
`HashMap<String, String>`, which is correct only because payload format 2.0
stringifies them. If one claim ever arrived as a number, a boolean or a list, the
decode of the *whole* request context would fail, `subject()` would return `None`,
and the request would be attributed to the development owner instead of being
refused — a write landing in the wrong partition, with no error anywhere. Nothing
in any environment observed this.

The second is the argument that makes the header safe at all. DR-0017 records
that `x-amzn-request-context` is not a security boundary and that what makes it
irrelevant is that API Gateway is the only route to the function and overwrites
the header on every request. Locally there was nothing to overwrite it, so the
claim had never been exercised in either direction.

This decision was taken as the second of four phases in a piece of work
establishing local verification of the deployed system, after DR-0020 did the
same for the DynamoDB table.

## Decision

A development-only binary crate, `crates/devgateway`, plays API Gateway and the
Lambda Web Adapter in front of the unmodified service. It listens on 3001,
forwards to the service on 3000 over plain HTTP, and reproduces the five
behaviours above and nothing else.

It is **outside `crates/server`**, because in the deployment it is outside. DR-0017
rests on the service being an ordinary axum binary that knows nothing about
Lambda; a stand-in compiled into the service would have destroyed the property it
exists to check.

Two of its behaviours are exact rather than approximate, and the rest of the
design follows from them.

**Any inbound `x-amzn-request-context` is discarded unconditionally**, before
anything else looks at the request, and the stand-in always writes its own. A rig
that let a caller's copy through would be a mirror in which the header is
forgeable, and would teach the opposite of what DR-0017 asserts.

**Claims are stringified the way API Gateway stringifies them**, including a list
claim — `cognito:groups` is the one that really arrives — rendered as `[a b]`
rather than as JSON. This is the first thing anywhere that demonstrates the
service's `HashMap<String, String>` is right.

Two modes. `local`, the default, is the mirror. `passthrough` forwards untouched
and is the absence of a mirror rather than a second one: it is what `just dev-api`
alone already does, forged header and all, and it exists so the difference can be
seen.

An `/api` request with no `Authorization` header is refused 401 and never reaches
the service. Any bearer value is otherwise accepted: a JWT is decoded without
being verified, and anything else is taken as the subject itself, so `Bearer
alice` and `Bearer bob` are two callers. Verifying a real token against the pool's
JWKS is a later phase; nothing here enforces anything.

Nothing about a fresh clone changes if the stand-in is never used. `just dev-api`
and `just dev-web` keep working on their own, in two terminals and with no
configuration, which is the same shape DR-0020 gave the DynamoDB recipes and
DR-0008's principle applied once more.

## Alternatives

**A mode inside `crates/server`.** An environment variable putting the service
into a "pretend there was an authorizer" mode would have needed no new crate. It
was rejected because it inverts what is being checked: the service would then
contain the thing standing in for the component in front of it, DR-0017's premise
would no longer hold, and the mode would be compiled into the deployed binary.

**AWS SAM CLI, or LocalStack.** Both emulate far more of AWS than is wanted here,
both are a second toolchain to install and pin in the devcontainer image, and both
are Python — which this container does not have and deliberately does not want.
Neither would have exercised the packaged binary the way the real Lambda does; SAM
would have run it in a container built to its own recipe rather than the one
`just deploy-api` produces.

**`cargo lambda watch`.** Rust, and closer in spirit. It emulates the Lambda
runtime API, which is the wrong layer: the runtime API is what the *adapter*
talks to, and the behaviours that are missing locally belong to API Gateway — the
route table, the preflight, the authorizer — not to the runtime. It would also
have meant running the service as a Lambda handler rather than as the axum binary
it is.

**An axum router as the route table.** The obvious way to write the stand-in, and
wrong in a way that matters: a router answers 405 for a method it has no handler
for, and an HTTP API answers 404, because the route does not exist. That is one of
the five behaviours being reproduced, so the route table is a hand-written match
rather than a router.

## Consequences

The route table and the CORS configuration are copies of `apigateway.tf`, kept in
step with it by hand. A method added to `local.api_methods` now has a second place
to follow it, in `crates/devgateway/src/edge.rs`. This is the same arrangement
`project` and `dynamo_table` in the `justfile` already have, and for the same
reason: nothing local can read Terraform. A drift between the two is visible the
first time the stand-in is used and invisible until then.

Three terminals when the rig is in use, and the `/api` proxy target moves out of
`Trunk.toml` into the `just` recipes — there are now two things it can point at,
and trunk appends a command-line backend to the file's entries rather than
overriding them, so a default in the file could not have been overridden. A bare
`trunk serve` run outside `just` therefore no longer proxies `/api`.

A `dev-web` bundle sends no `Authorization` header at all (DR-0008), so behind the
stand-in every `/api` call is a 401. That is not a defect: it is the local
reproduction of the deployment constraint that a bundle built without the two
Cognito variables cannot call the API. A browser session against the rig means
`just dev-web-auth` and a real token.

What this makes possible for the first time: two callers, one `curl` flag apart,
and a check that neither sees the other's items — the whole reason
`identity::Owner` exists, and something no local arrangement could observe before.

Reversing it costs the crate and four `justfile` recipes. Nothing depends on it:
the service, the SPA and the infrastructure are unchanged by its existence, which
is the property that made this shape worth choosing.
