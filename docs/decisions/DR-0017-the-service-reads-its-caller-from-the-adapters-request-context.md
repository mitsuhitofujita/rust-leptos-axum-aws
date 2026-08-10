# DR-0017: The service reads its caller from the adapter's request context

Status: accepted
Date: 2026-08-10

## Context

`docs/design/persistence.md` partitions every item by the owner's Cognito `sub`,
and states that the service must derive it from the validated token rather than
from anything the client sends. The IAM policy cannot express that constraint —
the function serves every user, so its permissions cover every partition — which
makes this the one place user isolation lives. Until now nothing needed it and
nothing did it.

Three things are already settled and shape what is left to decide. API Gateway's
JWT authorizer is the only enforcement point, and it validates the token before
the function is invoked (DR-0010). `crates/server` is an ordinary axum binary,
unmodified for Lambda, running behind the Lambda Web Adapter, which turns the
invocation event into an HTTP request on `127.0.0.1:3000`. And the adapter is
what stands between the event and the service: the service never sees the event,
so it never sees the authorizer's output in the place the event carries it,
`requestContext.authorizer.jwt.claims`.

The adapter does forward it. It serialises the event's request context as JSON
into an `x-amzn-request-context` header on the request it makes. `lambda_http`'s
`RequestContext` is `#[serde(untagged)]`, so an HTTP API's context is the whole
of that JSON rather than a variant inside it.

## Decision

The service takes the caller's `sub` from the `x-amzn-request-context` header,
decoding only `authorizer.jwt.claims.sub` out of it. It is an axum extractor,
`identity::Owner`, so a handler asks for the owner and cannot ask for anything
else; nothing takes an owner from a path, a query, or a body.

The token itself is neither read nor validated. The service holds no JWT library
and no public keys.

## Alternatives

- **Decode the access token from the `Authorization` header.** Rejected because
  it invites the mistake it resembles: reading claims out of a token the service
  has not verified. Doing it correctly means fetching and caching the pool's
  JWKS and verifying signature, issuer, audience and expiry — reimplementing the
  authorizer that has already run, in front of a function nothing can reach
  except through it.
- **Replace the adapter with the `lambda_http` runtime.** Rejected because it
  would make `crates/server` a Lambda program rather than an axum service that
  happens to be deployed as one. `just dev-api` would no longer run the same
  binary the deployment runs.
- **Accept an owner identifier as a request parameter.** Rejected outright:
  every account could then read and write every other account's partition. It is
  named here because it is the shape a handler falls into by accident.

## Consequences

User isolation is one extractor, in one file, and a handler that wanted to
bypass it would have to say so in its signature.

The header is not a security boundary and must not be mistaken for one. Anyone
who could reach the service directly could forge it; what makes that irrelevant
is that nothing can — API Gateway is the only route to the function, and it
overwrites the header on every request. If the service is ever exposed by any
other path, this decision has to be revisited before that path opens.

The dependency on the adapter is now load-bearing in a way `AWS_LWA_PORT` was
not. Removing the adapter, or a future version of it renaming or dropping the
header, breaks authentication rather than the deployment — and breaks it
quietly, because [DR-0018](DR-0018-the-service-runs-without-aws.md) makes a
missing header mean "development" rather than "reject". The header name is
therefore pinned in one constant, next to that reasoning.
