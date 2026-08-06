# DR-0008: The SPA receives its configuration at compile time, not at runtime

Status: accepted
Date: 2026-08-05

## Context

DR-0001 made the frontend a client-side rendered bundle delivered as static
files, with the API a separate service. That leaves the bundle with no way to
know where the API is: it is shipped to a CDN, and a CDN serves bytes rather than
answering questions about the environment they are running in.

The values the SPA needs are all produced by Terraform and published to SSM
(DR-0005): the API Gateway endpoint today, and the Cognito app client id and
hosted-UI domain once sign-in exists. The open question was not where they come
from but *when they reach the browser*.

Two further forces shaped it. Development must keep working with no configuration
at all — `trunk serve` proxies `/api` to the local API server, and that
single-origin arrangement is what keeps CORS out of development entirely. And
the deployed bundle is content-hashed by trunk, so anything baked into it changes
its filename.

## Decision

Configuration is read from SSM by `just deploy-web` and passed to
`trunk build --release` as environment variables. `crates/app` reads them through
`option_env!` into constants.

`API_BASE_URL` is the only one wired today. Every API call is an absolute path
joined to it, and an unset variable means the empty string — which leaves the
call relative, and therefore served by the trunk proxy. One code path is
correct in both places, and development needs no configuration to be supplied.

## Alternatives

**A `config.json` fetched by the SPA at startup.** The conventional answer, and
the one that lets a single bundle serve many environments. Rejected: it puts a
blocking round trip in front of the first API call, it needs its own cache rule —
a fourth case beside the hashed assets, `index.html` and `public/` — and it moves
a whole class of failure from build time to every visitor's browser, where a
stale or missing file is diagnosed by whoever happens to load the page. Its one
real advantage buys nothing here, because there is one environment.

**Routing `/api/*` through CloudFront as a second origin.** This removes the
question rather than answering it: the SPA becomes single-origin, no endpoint has
to be configured, and CORS disappears entirely. Rejected because it puts the API
behind a cache that the SPA and the API would then have to reason about together,
and it re-couples the two release cycles that DR-0001 separated in order to let
them fail and deploy independently. The independence is worth more than the
configuration step it would save.

**Writing the hostname into the source.** Rejected: it is account-specific, which
DR-0006 already refuses for the state bucket, and it cannot differ between
development and production without a conditional in the code.

## Consequences

Easy: no hostname appears in the frontend source; there is no runtime cost and no
startup round trip; the same code is single-origin locally and cross-origin in
production; and because the value lands inside a content-hashed bundle, changing
it invalidates its own cache entry rather than needing an invalidation to be
remembered. Cargo tracks variables reached through `option_env!`, so a changed
endpoint rebuilds the crate instead of silently reusing a bundle built against a
different one.

Hard, and accepted:

- **A configuration change is a rebuild and a full redeploy**, not an edit to a
  file in the bucket. The artefact in `dist/` is specific to the environment it
  was built for and cannot be promoted to another one.
- **Nothing secret can ever travel this path.** Every value passed is readable by
  anyone who downloads the bundle. That is fine for what is passed — the app
  client is public and holds no secret (PKCE replaces it) — and it is a hard
  limit the moment something genuinely secret is needed, at which point the value
  belongs behind the API rather than in the frontend.

Reversing this means changing `crates/app`, the `deploy-web` recipe and the
CloudFront cache rules together, since a runtime configuration file would need a
cache behaviour of its own.
