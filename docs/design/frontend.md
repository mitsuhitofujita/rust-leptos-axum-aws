# Frontend

Updated: 2026-08-07

## Purpose

The user interface: a Leptos single-page application, rendered entirely in the
browser, that fetches its data from the axum API over HTTP. It is delivered as
static files and runs no server-side code of its own (DR-0001).

## Structure

The application lives in `crates/app` and is compiled to
`wasm32-unknown-unknown`. It is a binary crate — trunk builds `src/main.rs`.

| File | Role |
| --- | --- |
| `crates/app/src/main.rs` | Installs the panic hook, mounts `App` to `<body>` |
| `crates/app/src/app.rs` | The router and every component |
| `crates/app/src/api.rs` | Calls to the API, returning `shared` types |
| `crates/app/src/auth.rs` | Sign-in against the Cognito hosted UI, and the token it yields |

Trunk's inputs sit at the repository root rather than inside the crate, because
the workspace manifest is virtual and cannot itself be a trunk target:

| File | Role |
| --- | --- |
| `index.html` | trunk entry point; points `data-trunk rel="rust"` at `crates/app/Cargo.toml` |
| `Trunk.toml` | dev server address and port, and the `/api` proxy |
| `style/main.css` | plain CSS, linked by `index.html` |
| `public/` | assets copied verbatim into `dist/public/` |

`trunk build` emits `dist/`: hashed `.wasm` and `.js`, hashed CSS, the copied
`public/` directory, and an `index.html` rewritten to reference them. `dist/` is
the deployable artefact and is not committed.

**Routing.** `leptos_router`'s `<Router>` wraps a `<Routes>` block declaring
`/` (`HomePage`) and `/about` (`AboutPage`), with a `NotFound` fallback.
Navigation uses `<A>`, which renders `aria-current="page"` on the active link —
the CSS styles that attribute rather than tracking the active route by hand.

**Data fetching.** `crates/app/src/api.rs` holds one async function per
endpoint. Each returns `Result<T, ApiError>` where `T` comes from
`crates/shared`. `ApiError` separates `Unauthorized` from everything else,
because 401 is the one failure a visitor can act on; the rest are transport
failures, unexpected statuses and decode failures carried as messages the UI can
render. Every request attaches the access token when the tab holds one.
Components load them with `LocalResource` inside `<Suspense>`; the error branch
is rendered, never swallowed.

`LocalResource`, not `Resource`: the browser fetch future is not `Send`, and in
a CSR build there is no server to run it on.

**Authentication.** `crates/app/src/auth.rs` implements Authorization Code Flow
with PKCE against the Cognito hosted UI by hand — no auth library, no AWS SDK
(DR-0010). `App` settles an `AuthState` signal once at mount and provides it
through context: `Loading` until the callback has been dealt with, then
`Disabled`, `SignedOut`, `SignedIn` or `Error`. The header renders a control from
it, and nothing at all when the state is `Disabled`.

The access token, its expiry and the email live in `sessionStorage`. No refresh
token is kept: an expired session sends the visitor back to the hosted UI. A 401
drops the token and offers a fresh sign-in rather than redirecting, which would
loop for any 401 a new token cannot fix.

The redirect URI is not configured — it is `window.location.origin` with a
trailing slash, so it is the CloudFront domain in a deployed build and
`http://localhost:8080/` under `trunk serve`, both already registered on the app
client.

`HomePage`'s `LocalResource` reads the auth signal in its source closure. That is
load-bearing rather than incidental: the state settling is what must re-run the
fetch, or the first request after a sign-in leaves before the token is stored.

**Compile-time configuration** is three environment variables, each read once
through `option_env!` into a constant, and each with an unset value that means
something workable rather than something broken (DR-0008). `just deploy-web`
resolves all three from SSM; `just dev-web-auth` resolves the two Cognito ones
around `trunk serve`.

| Variable | Unset means | Read by |
| --- | --- | --- |
| `API_BASE_URL` | the empty string, so every call stays relative and the trunk proxy serves it | `api.rs` |
| `COGNITO_CLIENT_ID` | sign-in is not configured | `auth.rs` |
| `COGNITO_HOSTED_UI_DOMAIN` | the same | `auth.rs` |

Either Cognito variable empty disables sign-in entirely: no control, no
`Authorization` header, and the local API — which validates nothing — answers
anyway. That is what keeps development needing no configuration at all.

## Interfaces

**Consumes** `GET /api/greeting` → `shared::Greeting`, as an absolute path joined
to `API_BASE_URL`, carrying a bearer token when there is one. No API hostname
appears in the source; the origin arrives at build time, or not at all in
development.

**Consumes** the Cognito hosted UI's `/oauth2/authorize`, `/oauth2/token` and
`/logout`, at the domain `COGNITO_HOSTED_UI_DOMAIN` names.

**Depends on** `leptos` (feature `csr`), `leptos_router` (default features — it
has no `csr` feature), `gloo-net` (features `http`, `json`) for fetch,
`console_error_panic_hook`, and `shared`. The sign-in flow adds `web-sys`
(features `Window`, `Location`, `History`, `Storage`, `Crypto`,
`UrlSearchParams`), `js-sys`, `wasm-bindgen`, `sha2` and `base64` for the PKCE
challenge, and `serde` with `serde_json` for the token response. Every one was
already in `Cargo.lock` at the version the workspace now names, so adding them
moved nothing — `wasm-bindgen` least of all, which trunk keys its CLI download
off (DR-0003).

**Exposes** nothing to other crates.

## Constraints

- No API hostname is written into the source. Calls are absolute paths joined to
  `API_BASE_URL`, which is supplied at build time and not fetched at runtime, so
  the development proxy and the production origin are both settled outside the
  code — DR-0008.
- The dev server proxies `/api` to `127.0.0.1:3000`, so development is
  single-origin and CORS never arises. Production is cross-origin and requires
  CORS on the API — DR-0001.
- Every request under `/api` needs a Cognito access token in an `Authorization`
  header, which `auth.rs` obtains from the hosted UI and `api.rs` attaches — but
  only in a build configured for it. An unconfigured build sends no header, which
  the local API accepts and API Gateway does not, so a deployed bundle built
  without the two Cognito variables renders a 401 where the greeting belongs.
  Configuration is what distinguishes the two, not code — DR-0010.

- The token is never validated in the browser. Expiry is checked so an expired
  one is not sent, and the id token's `email` claim is decoded to label the
  header, but neither is a signature check: API Gateway's authorizer is the
  security boundary — DR-0010.
- Deep links are router paths, not files. `trunk serve` serves `index.html` for
  unknown paths, and the production host must be configured to do the same, or
  reloading on any non-root route fails — DR-0001.
- CSS is plain and hand-written. No framework, no Node.js, no npm anywhere in
  the build.
- The framework stays inside this crate; `shared` and `server` never import
  Leptos — DR-0002.
