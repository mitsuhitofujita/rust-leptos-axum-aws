# Sign in with Cognito, so the API answers the SPA

Status: in progress
Started: 2026-08-06
Branch: main

## Request

Close the gap the Terraform setup deliberately left open: the deployed SPA
renders a 401 where the greeting belongs, because the HTTP API puts a JWT
authorizer in front of `/api/{proxy+}` and `crates/app` sends no `Authorization`
header. Make a signed-in visitor see the greeting.

The stated reason for the shape of the work is that the infrastructure half
already exists. The `identity` layer runs a Cognito user pool with Google as an
identity provider and a public, secretless app client whose callback and logout
URLs already cover both the CloudFront domain and `http://localhost:8080`, and
it publishes `app_client_id` and `hosted_ui_domain` to SSM where nothing reads
them yet. What is missing is entirely inside `crates/app`.

Sign-in must be off by default in local development, so `trunk serve` keeps
needing no configuration.

### Clarifications

Asked and answered on 2026-08-06, during planning, before any code was written.

- **Local development.** Unset `COGNITO_*` variables mean no sign-in control and
  no header, in exactly the way an unset `API_BASE_URL` means a relative call
  today (DR-0008). A separate `just` recipe resolves the real values from SSM for
  when the flow itself is being tested.
- **Tokens.** Access token in `sessionStorage`. No refresh token is kept.
  Expiry is handled by sending the visitor back to the hosted UI.
- **Verification is local only.** No `just deploy-web`, no S3 upload, no
  invalidation — deploying stays the user's call. AWS credentials in this
  devcontainer have expired, so anything that reads SSM needs `aws login` first.

Arriving later.

- **2026-08-07 — write this Work Log.** The plan was produced in plan mode, which
  could write only its own file, so the log the process calls for did not exist.
  Create it from the template in `docs/README.md` before the implementation
  starts.

## Interpretation

**What is being asked.** Implement Authorization Code Flow with PKCE against the
Cognito hosted UI inside `crates/app`, attach the resulting access token to API
calls, and give the header somewhere to show who is signed in — so that the
authorizer that has so far only ever been observed rejecting is observed
accepting.

**What is out of scope.**

- Any Terraform change. The app client, its callback and logout URLs, its
  scopes, and the authorizer's audience are already correct for this flow;
  nothing under `infra/` is touched.
- `crates/server`. API Gateway is the enforcement point. Adding JWT validation to
  the service would duplicate it and break the single-origin local setup, and is
  worth revisiting only if the API ever gains a second caller.
- Deploying. The bundle is not rebuilt or uploaded in this unit of work, which
  puts a ceiling on what can be proven and is why the plan reaches the
  authorizer by calling the deployed API directly with a locally obtained token
  rather than by redeploying.

**What is assumed.**

- The durable layer is the specification. `docs/design/deployment.md` names the
  missing `Authorization` header as the one remaining gap, and DR-0008 fixes the
  shape any new configuration takes: compile-time, through `option_env!`, unset
  meaning disabled.
- The Google-issued email is present in the id token. The header falls back to a
  generic signed-in label when it is not.
- Every dependency the flow needs is already resolved in `Cargo.lock` at the
  version the plan names, so adding them pulls nothing new in — and, importantly,
  does not move the `wasm-bindgen` version trunk keys its CLI download off, which
  is DR-0001's last recorded pitfall.
- The id token is decoded for display only. It is not verified; the API is the
  security boundary.

## Plan

Taken from `docs/plans/api-work-log-compiled-fox.md`, which carries the full
reasoning. The design it settles, since that file is deleted once this log
absorbs it:

- **Hand-written flow, no library.** A new `crates/app/src/auth.rs`. No AWS SDK
  and no Amplify: the flow is two URLs and one form POST, and every JavaScript
  library for it is larger than the code it replaces.
- **Two compile-time variables**, `COGNITO_CLIENT_ID` from
  `/<project>/identity/app_client_id` and `COGNITO_HOSTED_UI_DOMAIN` from
  `/<project>/identity/hosted_ui_domain`, both read through `option_env!` into
  constants exactly as `api.rs` reads `API_BASE_URL`. Either one empty means
  sign-in is not configured: no control, no header, and the local API — which
  checks nothing — keeps answering.
- **The redirect URI is not configured.** It is `window.location.origin` plus a
  trailing slash, computed at runtime, which is `https://<cloudfront>/` in
  production and `http://localhost:8080/` locally. Both are already registered on
  the app client, so there is one less build variable and no way for the two to
  drift.
- **The access token is what is sent, not the id token.** The authorizer's
  `audience` is the app client id, which a Cognito access token carries as
  `client_id`.
- **Session state in `sessionStorage`**, cleared when the tab closes:
  `auth.access_token`, `auth.expires_at` (ms), `auth.email`, and
  `auth.pkce_verifier` / `auth.state` for the duration of the redirect only.
  Expiry is checked against `expires_at` before a call. An unexpected 401 clears
  the session and asks the visitor to sign in again rather than redirecting on
  its own — an automatic redirect on any 401 is a loop whenever the API returns
  one for a reason other than the token.

The steps:

1. **Workspace dependencies** — `web-sys` and `js-sys` at `0.3.103`,
   `wasm-bindgen` at `0.2.126`, `base64 = "0.22"`, `sha2 = "0.10"`,
   `serde_json = "1.0"` in the root `Cargo.toml`, added to `crates/app` with
   `serde` and the `web-sys` features `Window`, `Location`, `History`, `Storage`,
   `Crypto`, `UrlSearchParams`.
2. **`crates/app/src/auth.rs`** — the constants and an `is_configured()`
   predicate; `begin_sign_in()` generating a 32-byte PKCE verifier and a 16-byte
   `state` from `crypto.getRandomValues`, base64url-no-pad, with the challenge as
   base64url(SHA-256(verifier)); `complete_sign_in()` checking `state`,
   form-POSTing `grant_type=authorization_code` to `/oauth2/token` with no client
   secret, storing token, expiry and email, and cleaning the URL with
   `history.replaceState` so a reload cannot replay a spent code;
   `access_token()`; `sign_out()`; and a `decode_jwt_claims` helper.
3. **`crates/app/src/api.rs`** — attach `Authorization: Bearer <token>` when
   there is one, and give 401 a distinguishable error so the UI can offer to sign
   in again rather than printing a status code.
4. **`crates/app/src/app.rs`** — an auth-state signal (`Loading`, `SignedOut`,
   `SignedIn`, `Disabled`, `Error`) in context, settled once at mount, and a
   sign-in / sign-out control in the header. The load-bearing detail:
   `HomePage`'s `LocalResource` must read the auth signal in its source closure,
   or the first render after a sign-in fetches without a token, renders the 401,
   and never retries.
5. **`style/main.css`** — style the control alongside the existing header rules.
   Plain CSS, per `docs/design/frontend.md`.
6. **`justfile`** — a `dev-web-auth` recipe resolving both values through `_ssm`
   around `trunk serve`, and the same two variables beside `API_BASE_URL` in
   `deploy-web`, whose comment about nothing in `crates/` reading them stops
   being true.
7. **`docs/design/frontend.md`** — data-fetching/authentication, the
   compile-time variables, and the replacement of the constraint saying the SPA
   obtains no token. Confirmed by the user before it lands.
8. **`docs/design/deployment.md` and `docs/design/index.md`** — two more rows in
   "Configuring the SPA", removal of the top note and of the "SPA sends no
   `Authorization` header yet" constraint, and the new record in the index.
9. **DR-0010** — hosted UI with PKCE by hand, access token in `sessionStorage`,
   no refresh token, no automatic redirect on 401, sign-in disabled when
   unconfigured. Rejecting: keeping the refresh token, memory-only tokens,
   `localStorage`, and an off-the-shelf auth library. This is the record that
   makes this log safe to delete.

## Progress

### 2026-08-06

Request received and planned in plan mode. Read `docs/design/deployment.md`,
`frontend.md`, `index.md`, DR-0001 and DR-0008, and the `identity` and `api`
layers under `infra/`, and confirmed against the tree that the gap is where
`deployment.md` says it is: `crates/app/src/api.rs` reads `API_BASE_URL` through
`option_env!` and sends no header, and nothing in `crates/` mentions Cognito.

Three questions raised — local development behaviour, where tokens live, and how
far verification goes — and answered under Clarifications. The verification
answer is what caps this work: local only, no deploy, so the end-to-end proof has
to come from calling the deployed API by hand.

Plan written to `docs/plans/api-work-log-compiled-fox.md`. No code written.

### 2026-08-07

**Step 0 — this Work Log opened**, a day after the plan, because plan mode could
write only the plan file. The Request above restates the ask in English and the
Plan section carries the design settled yesterday, so this log stands on its own
once the plan file is deleted.

State of the tree: nothing from steps 1–9 exists. `crates/app/src/` is still
`api.rs`, `app.rs`, `main.rs`; the root `Cargo.toml` has none of the new
dependencies; the `justfile` has no `dev-web-auth`; `docs/decisions/` stops at
DR-0009. The only uncommitted change is the plan file itself, alongside the two
retired plans staged for deletion from the previous unit of work.

Next action: step 1.

**Steps 1–6 implemented**, in the order planned, with no change to the design
settled yesterday. What is worth recording beyond "it was done":

*Step 1.* The assumption held exactly. `Cargo.lock` grew by seven lines, all of
them edges from `app` to crates already in the graph; no version moved, and
`wasm-bindgen` in particular stayed at `0.2.126`, so trunk's CLI download is
untouched.

*Step 2.* `auth.rs` came out as planned. Two details the plan did not name:
`hosted_ui()` supplies the `https://` scheme, because SSM publishes the domain
bare as `<prefix>.auth.<region>.amazoncognito.com`; and `clean_url()` replaces the
URL with `location.pathname` alone, which drops the query as intended and the
fragment with it — harmless, since the router uses paths.

*Step 3.* `fetch_greeting` now returns `Result<Greeting, ApiError>` rather than
`Result<Greeting, String>`, with `Unauthorized` split out. This contradicts a
sentence in `docs/design/frontend.md` that said every API function returns
`Result<T, String>`, so that document was updated rather than worked around.

*Step 4.* The load-bearing detail the plan flagged was handled as described —
the resource's source closure reads the auth signal. Handling the 401 turned out
to need a second guard the plan had not anticipated: the effect that drops the
token on a 401 writes to the auth signal, which re-runs the resource, which fails
the same way. Left ungated that is the very loop the no-redirect-on-401 rule
exists to avoid, arriving by a different route. The transition is therefore
allowed only *from* `SignedIn`, so it happens at most once per session and a 401
with no token to blame writes nothing.

*Step 6.* `deploy-web`'s comment about nothing in `crates/` reading the two
values was replaced rather than deleted; the recipe now sets all three variables.

**Steps 7–8 drafted, awaiting confirmation.** `frontend.md`, `deployment.md` and
`index.md` are edited in the working tree but are overwrites, which
`docs/README.md` reserves for a human to confirm. The Retirement box stays
unticked until they do.

**Step 9 written**: `DR-0010-the-spa-signs-in-through-the-hosted-ui-by-hand.md`.
It carries the four rejected alternatives, and the two non-obvious constraints
the code cannot state for itself — the computed redirect URI and the
no-redirect-on-401 rule.

Verification run: see below. The client half is proven as far as this container
allows; the authorizer half is not, for want of both a browser and credentials.

## Verification

Planned, not yet run. Recorded here so the ceiling is visible before the work
starts rather than discovered at the end.

Steps 1–6 are checkable in this container with no credentials:

```sh
just fmt-check && just check && just lint && just test && just build
```

### What was actually run, 2026-08-07

**The command suite above: passed.** `fmt-check` clean after one `just fmt`;
`check` and `lint` clean for both the native and the `wasm32-unknown-unknown`
target; `test` runs no tests, as before this work; `build` produced a release
bundle.

**One check the plan did not think of, which turned out to be the most useful
thing available here.** The compile-time gating is observable in the artefact
itself, without a browser: build twice and look at what is in the `.wasm`.

```sh
trunk build --release                       # unconfigured
grep -c amazoncognito dist/*.wasm           # 0

COGNITO_CLIENT_ID=… COGNITO_HOSTED_UI_DOMAIN=… trunk build --release
grep -ao 'oauth2/authorize\|code_challenge_method\|…amazoncognito.com' dist/*.wasm
```

The unconfigured bundle contains no Cognito hostname, no `oauth2/authorize`, no
`oauth2/token` — the strings are not merely unreachable, `option_env!` means they
are absent. The configured one contains the client id, the domain and both
endpoints. That is direct evidence for the property this work most needed to
preserve: an unconfigured build has no sign-in in it at all. `dist/` was rebuilt
unconfigured afterwards, so the working tree's bundle is the development one.

Cargo's tracking of `option_env!` was confirmed incidentally: changing the
variables re-checked the crate rather than reusing the previous result, which is
the mechanism DR-0008 relies on.

**What could not be run here, and why.** Everything below this line in the plan
needs a browser, and this devcontainer has none — no chromium, no firefox, no
node. The DOM-level checks are therefore untouched, not failed: whether the
control appears, whether the redirect lands, whether `?code=` is cleared, whether
the header shows the address, whether the request carries the token. The
authorizer check needs credentials as well, which have expired.

So the honest summary is: the flow compiles, is lint-clean, and is provably
absent from an unconfigured build. Not one line of it has been executed.

**Unconfigured path — no credentials needed.** `just dev-api` in one shell,
`trunk serve` in another, then `http://localhost:8080`: the greeting renders, no
sign-in control appears, and the network panel shows `/api/greeting` carrying no
`Authorization` header. This is the regression that matters most — the
zero-configuration development path DR-0008 records must not have moved.

**Configured path — needs the two values.** `aws login` then `just dev-web-auth`,
or both variables exported by hand. Then: the control appears; it lands on the
hosted UI and on Google; the return to `http://localhost:8080/` clears `?code=`
from the address bar; the email shows in the header; the greeting renders; the
request carries `Authorization: Bearer …`; a reload keeps the session; sign-out
clears it.

What this does not prove: locally the request goes to the axum server, which
ignores the header. It verifies the client half only.

**The authorizer half, without deploying.** Copy the access token out of
`sessionStorage` after signing in locally and call the deployed API directly:

```sh
endpoint="$(just _ssm api/api_endpoint)"
curl -i -H "Authorization: Bearer ${TOKEN}" "${endpoint%/}/api/greeting"
```

200 with the greeting JSON is the end-to-end proof that the token the SPA obtains
is one API Gateway accepts. The same call without the header still returning 401
confirms the authorizer is doing its job. Only the SSM lookup needs credentials;
the endpoint can be pasted instead.

Left unproven by design, since the user reserved deploying: that a bundle built
with the two variables and uploaded to S3 signs in from the CloudFront origin.
The redirect URI is computed at runtime and the CloudFront callback is already
registered, so nothing about the deployed path is untested for a reason other
than not having been run — but it has not been run.

## Retirement

- [ ] Design Documents updated — `frontend.md`, `deployment.md`, `index.md`.
      Drafted and in the working tree; **awaiting the user's confirmation**,
      which is what this box waits on: `docs/README.md` makes a human the owner
      of any design-document overwrite.
- [x] Decision Records written — DR-0010.
- [x] Non-obvious knowledge preserved. The rejected token-storage shapes and the
      case against an auth library are DR-0010's Alternatives. The
      runtime-computed redirect URI and the no-redirect-on-401 rule are in its
      Decision, and repeated as constraints in `frontend.md` and
      `deployment.md`. The resource-source-closure trap is in `frontend.md`,
      stated as load-bearing, and in the comment above the resource itself; the
      loop it can create through the 401 effect — found during step 4, not
      predicted — is recorded in Progress and guarded in the code.
- [x] No durable document depends on this log. `grep -rn` over `docs/design/`
      and `docs/decisions/` for `2026-08-06-spa-sign-in-with-cognito` and
      `api-work-log-compiled-fox` returns nothing.

Two things are outstanding, and both are the user's to decide:

- **The deployed page is still broken** until `just deploy-web` runs. That was
  reserved deliberately, so it is not a defect in this work — but the gap
  `deployment.md` described is closed in the repository, not on CloudFront.
- **Nothing has been executed.** Before this log is deleted, the flow deserves
  one pass through a browser — `just dev-web-auth`, then the curl against the
  deployed API — since the whole of it is unrun code.

`docs/plans/api-work-log-compiled-fox.md` has been absorbed into the Plan section
above and can be deleted. It was never committed, so its deletion is final; left
in place for the user to dispose of.
