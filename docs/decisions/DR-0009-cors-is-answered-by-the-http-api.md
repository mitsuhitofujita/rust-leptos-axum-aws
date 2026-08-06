# DR-0009: CORS is answered by the HTTP API, so `/api` routes are declared per method

Status: accepted
Date: 2026-08-05

## Context

DR-0001 recorded the absence of a CORS layer in `crates/server` as a gap the
deployment work has to close, not as a decision that it is unnecessary. The SPA
is served from a CloudFront domain and the API from an API Gateway domain, so
every call the browser makes is cross-origin and something has to answer for it.

The API sits behind a JWT authorizer validating Cognito access tokens. That
matters more than it first appears, because of when a preflight exists at all: a
`GET` with no `Authorization` header is a CORS-simple request and triggers no
preflight, while the same `GET` carrying a token does. The preflight therefore
arrives for the first time on the day sign-in starts working — and it carries no
`Authorization` header of its own, because a preflight never does.

An HTTP API answers preflight itself, ahead of any authorizer, but **only for an
`OPTIONS` request that no route matches**. An `ANY` route matches `OPTIONS`.

## Decision

CORS is configured on the `aws_apigatewayv2_api`, with the CloudFront domain as
the allowed origin. `crates/server` stays free of a CORS layer, so local
development remains single-origin through the trunk proxy.

`/api/{proxy+}` is therefore declared once per method rather than once as `ANY`,
from a `local.api_methods` list that `cors_configuration.allow_methods` also
derives from, so the two cannot drift. Leaving `OPTIONS` unrouted is what lets
the HTTP API answer the preflight.

## Alternatives

**A `tower-http` CorsLayer in `crates/server`.** The obvious place to put CORS,
and the one DR-0001 was pointing at. Rejected because it cannot work behind the
authorizer: with an `ANY` route, `OPTIONS` is rejected with 401 before the Lambda
is ever invoked, so the service would be answering a preflight it never receives.
It would also carry the allowed origin in the API's code rather than in the layer
that reads the CloudFront domain from SSM, and it would give local development a
CORS path it otherwise does not need.

**Keeping `ANY` and adding an unauthenticated `OPTIONS /api/{proxy+}` route.**
Rejected: a matching route still wins over the built-in answer, so this routes
every preflight to the Lambda and lands back in the problem above, paying a cold
start for a request API Gateway would have answered for free.

**Keeping `ANY` alone.** Not a trade-off, an error, and measured as one:
`OPTIONS /api/greeting` returned 401 where a preflight must return 2xx or the
browser blocks the request it precedes.

## Consequences

Easy: preflight is answered ahead of the authorizer and without invoking the
Lambda, so it costs nothing and cannot be rejected for lacking a token; the
allowed origin lives in the layer that already reads the CloudFront domain; and
`crates/server` stays a plain axum service, usable by any future client without
carrying one deployment's origin list.

Hard: a new HTTP method is now an infrastructure change — it goes in
`local.api_methods` — where `ANY` would have needed none. `{proxy+}` still means
a new *endpoint* needs no change, which is the more frequent case by far.

Worth stating because it is not visible from reading the code: an `ANY` route
looks correct for exactly as long as the SPA is unauthenticated. The bug it
carries surfaces on the day the first `Authorization` header is sent, in whatever
work is happening then, against code that will look like the culprit and is not.

Reversing this is one Terraform apply.
