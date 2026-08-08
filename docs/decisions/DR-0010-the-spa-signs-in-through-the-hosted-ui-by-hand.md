# DR-0010: The SPA signs in through the hosted UI, with a PKCE flow written by hand

Status: accepted
Date: 2026-08-07

## Context

The stack was fully applied and both artefacts deployed, and the deployed page
still did not work: the HTTP API puts a JWT authorizer in front of
`/api/{proxy+}`, `crates/app` sent no `Authorization` header, and every call to
`GET /api/greeting` returned 401. The `identity` layer already ran a Cognito user
pool with Google as an identity provider and published the two values a client
needs — `app_client_id` and `hosted_ui_domain` — which nothing read. The missing
half was entirely in the frontend.

Four forces shaped how it was filled.

The client cannot hold a secret. It is a WASM bundle any visitor downloads, so
the app client is public and PKCE is what stands in for a secret — a property the
`identity` layer had already committed to.

Development must keep needing no configuration. DR-0008 established the shape:
values arrive as compile-time variables, and an unset variable is a working
default rather than a failure. The local axum server checks no token, so a
development build has nothing to sign in to.

The bundle is a static file on a CDN. There is no server-side session to keep a
token in, and no back end of its own to broker one.

And the tokens are handled by code that runs in the visitor's browser, where
every storage choice trades one exposure against another.

## Decision

Authorization Code Flow with PKCE against the Cognito hosted UI, implemented by
hand in `crates/app/src/auth.rs`.

- **Two compile-time variables**, `COGNITO_CLIENT_ID` and
  `COGNITO_HOSTED_UI_DOMAIN`, read through `option_env!` exactly as `api.rs`
  reads `API_BASE_URL` (DR-0008). Either one empty means sign-in is not
  configured: no control is rendered, no header is attached, and the local API
  keeps answering. `just dev-web-auth` supplies the real values from SSM when the
  flow itself is being worked on.

- **The redirect URI is computed, not configured.** It is
  `window.location.origin` plus a trailing slash, which is `https://<cloudfront>/`
  in a deployed build and `http://localhost:8080/` under `trunk serve`. Both are
  already registered on the app client, so there is one less build variable and
  no way for the value sent to `/oauth2/authorize` and the one sent to
  `/oauth2/token` to disagree.

- **The access token is what is sent.** The authorizer's `audience` is the app
  client id, which a Cognito access token carries as `client_id`. The id token is
  decoded once for the `email` claim, to label the header, and dropped. That
  decode reads the payload segment without checking the signature; nothing is
  authorised by it, because API Gateway is the security boundary.

- **The session lives in `sessionStorage`** — the access token, an `expires_at`
  in milliseconds, and the email — plus the PKCE verifier and the `state` for the
  duration of one redirect. Expiry is checked before every call, and an expired
  session is dropped rather than sent.

- **No refresh token is kept.** When the access token expires the visitor goes
  back to the hosted UI, where Cognito's own session usually returns them without
  a further prompt.

- **A 401 never redirects on its own.** It drops the token and offers a fresh
  sign-in. The transition happens only from a signed-in state, so a 401 that
  arrives with no token to blame changes nothing.

`crates/server` is not involved. API Gateway enforces the token; adding
validation to the service would duplicate the check and break the single-origin
local setup that keeps CORS out of development.

## Alternatives

**An off-the-shelf auth library** — AWS Amplify, `oidc-client-ts`, or one of the
Rust wrappers. Rejected: the flow is two URLs and one form POST, and every
JavaScript option is larger than the code it replaces — Amplify by a wide margin,
in a bundle whose whole point is being small enough to ship as a static file.
They also reintroduce a JavaScript dependency and, with it, the npm toolchain
this project has kept out of the build entirely. `auth.rs` is one module with no
dependency the crate did not already resolve.

**Keeping the refresh token.** The conventional choice, and it is what makes a
session survive past an hour without a visible redirect. Rejected because a
refresh token in browser storage is a far worse thing to lose than an access
token: it is long-lived and it mints new access tokens, so it turns a scripting
flaw into durable access instead of a short window. The hosted UI's own session
already softens the cost, since the return trip is usually silent. Revisit if the
redirects turn out to be visible in practice.

**Tokens in memory only.** The safest storage there is — nothing to steal from
disk, nothing surviving the page. Rejected because every reload signs the visitor
out, and a reload is not rare on a page whose deep links are router paths. With
no refresh token there would be nothing to recover the session from either, so
the two rejections compound. `sessionStorage` is the shortest lifetime that still
survives a reload.

**`localStorage`.** Rejected: it persists across tabs and restarts, which widens
the window a stolen token is useful in without buying anything the visitor asked
for. `sessionStorage` ends the session with the tab, which is closer to what
closing the page is understood to mean.

**Redirecting to the hosted UI automatically on any 401.** Tempting, because it
makes an expired token invisible. Rejected: it is an infinite loop the moment the
API returns 401 for a reason a fresh token cannot fix. The visitor is asked
instead.

**Validating the JWT in `crates/server` as well.** Rejected as duplication with
one enforcement point already in place, and it would either break local
development or need a bypass that is itself the risk. Worth revisiting only if
the API ever gains a caller that does not arrive through API Gateway.

## Consequences

Easy: a signed-in visitor sees the greeting, which is what the stack was
missing; the frontend gained no npm dependency and no framework; the deployed
bundle and the development build run the same code path, because unconfigured is
a working state rather than a broken one; and the redirect URI cannot drift from
what the app client registers, because it is derived from wherever the page is
being served.

Hard, and accepted:

- **The session ends when the tab closes, and again roughly every hour**, at
  which point the visitor is sent back through the hosted UI. Usually silent,
  occasionally not.
- **The access token is readable by any script that runs on the page.** That is
  the cost of a browser-held token; keeping the window short is what is done
  about it, and the refresh token — the thing genuinely worth stealing — is never
  stored at all.
- **The flow is this project's code to maintain.** A change in Cognito's endpoints
  or parameters lands here rather than in a dependency's next release. The
  surface is small, and it is a standard flow, but it is not somebody else's.
- **Nothing verifies the token in the SPA**, so a build pointed at the wrong pool
  fails at the API rather than at sign-in, as a 401 the visitor is asked to
  recover from.

Reversing the storage decision means changing `auth.rs` alone. Reversing the
hand-written flow means changing `auth.rs`, the crate's dependencies, and — for
any JavaScript option — the claim in `docs/design/frontend.md` that no Node.js
appears anywhere in the build.
