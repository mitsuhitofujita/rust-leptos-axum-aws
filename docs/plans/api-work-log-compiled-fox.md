# Sign-in with Cognito, so the API answers the SPA

## Context

The AWS stack is fully applied and both artefacts are deployed, but the deployed
page does not work. `docs/design/deployment.md` names the reason as the one
remaining gap: the HTTP API puts a JWT authorizer in front of `/api/{proxy+}`,
and `crates/app/src/api.rs` sends no `Authorization` header, so every call to
`GET /api/greeting` returns 401 and the SPA renders that status where the
greeting belongs.

Everything the missing half needs already exists. The `identity` layer runs a
Cognito user pool with Google as an identity provider and a public, secretless
app client whose callback and logout URLs already list both the CloudFront domain
and `http://localhost:8080`. It publishes `/<project>/identity/app_client_id`
and `/<project>/identity/hosted_ui_domain` to SSM, read by nothing so far. The
API's authorizer is configured to accept an access token from that client. What
is missing is entirely inside `crates/app`: the redirect to the hosted UI, the
PKCE exchange, somewhere to keep the token, and the header.

The intended outcome is that a signed-in visitor sees the greeting, and that the
mechanism is off by default in local development so `trunk serve` keeps needing
no configuration.

**Scope note.** The user chose local verification only — no `just deploy-web`,
no S3 upload, no invalidation. The final proof that the authorizer accepts the
token is therefore reached by calling the deployed API directly with a token the
SPA obtained (step 9), not by redeploying the bundle. Deploying stays their call.

### Answers already given

- **Local development:** unset `COGNITO_*` variables mean no sign-in UI and no
  header, exactly as unset `API_BASE_URL` means a relative call today (DR-0008).
  A separate recipe resolves the real values from SSM when the flow itself is
  being tested.
- **Tokens:** access token in `sessionStorage`, no refresh token kept, expiry
  handled by returning to the hosted UI.
- **Verification:** local only; AWS credentials in this devcontainer have
  expired, so anything reading SSM needs `aws login` first.

## Step 0 — open the Work Log

Plan mode allowed only this file to be written, so the Work Log the `/work-log`
skill calls for does not exist yet. Create
`docs/work/2026-08-06-spa-sign-in-with-cognito.md` from the template in
`docs/README.md`, with the Request restated in English, the Interpretation and
assumptions below, and this plan. Append to Progress as the steps land.

## Design

Authorization Code Flow with PKCE against the Cognito hosted UI, implemented by
hand in a new `crates/app/src/auth.rs`. No AWS SDK and no Amplify: the flow is
two URLs and one form POST, and every JavaScript library for it is larger than
the code it replaces.

**Configuration**, following DR-0008 and the shape of `API_BASE_URL`:

| Variable | Source | Read by |
| --- | --- | --- |
| `COGNITO_CLIENT_ID` | `/<project>/identity/app_client_id` | `crates/app/src/auth.rs` |
| `COGNITO_HOSTED_UI_DOMAIN` | `/<project>/identity/hosted_ui_domain` | `crates/app/src/auth.rs` |

Both read through `option_env!` into constants, matching `api.rs` exactly. Either
one empty means sign-in is not configured: no button, no header, and the local
API — which checks nothing — keeps answering. This is what preserves the
zero-configuration development property DR-0008 records.

**The redirect URI is not configured.** It is `window.location.origin` plus a
trailing slash, computed at runtime, which is `https://<cloudfront>/` in
production and `http://localhost:8080/` locally — both already registered on the
app client. One less build variable, and no way for the two to drift.

**Send the access token, not the id token.** The authorizer's `audience` is the
app client id, which a Cognito access token carries as `client_id`. The id token
is decoded once for the email, to show who is signed in, and then discarded — a
display-only decode, not a verification; the API is the security boundary.

**Session state** in `sessionStorage`, cleared when the tab closes:

| Key | Value |
| --- | --- |
| `auth.access_token` | the bearer token |
| `auth.expires_at` | `Date.now()` at exchange plus `expires_in`, in ms |
| `auth.email` | from the id token, for the header UI |
| `auth.pkce_verifier`, `auth.state` | during the redirect only, removed on return |

Expiry is checked against `expires_at` before a call; an expired session is
cleared and the visitor is asked to sign in again. An unexpected 401 does the
same rather than redirecting on its own — an automatic redirect on any 401 is a
loop whenever the API returns one for a reason other than the token.

## Steps

1. **Workspace dependencies** — add to `[workspace.dependencies]` in the root
   `Cargo.toml`: `web-sys` and `js-sys` at `0.3.103`, `wasm-bindgen` at
   `0.2.126`, `base64 = "0.22"`, `sha2 = "0.10"`, `serde_json = "1.0"`. Every one
   is already resolved in `Cargo.lock` at those versions, so nothing new is
   pulled in and — importantly — the `wasm-bindgen` version trunk keys its CLI
   download off does not move (DR-0001's last pitfall). Add them to
   `crates/app/Cargo.toml`, with `serde` for the token-response struct and
   `web-sys` features `Window`, `Location`, `History`, `Storage`, `Crypto`,
   `UrlSearchParams`.

2. **`crates/app/src/auth.rs`** — the whole flow, in one module:
   - the two `option_env!` constants and an `is_configured()` predicate;
   - `begin_sign_in()`: 32 random bytes from `crypto.getRandomValues` as the
     PKCE verifier and 16 more as the `state`, both base64url-no-pad; the
     challenge is base64url(SHA-256(verifier)); store both, build the
     `/oauth2/authorize` query with `UrlSearchParams`, assign to
     `location.href`;
   - `complete_sign_in()`: if `code` is in `location.search`, compare `state`
     against the stored value, form-POST `grant_type=authorization_code` with the
     verifier to `/oauth2/token` (no client secret — the client is public), store
     the access token, expiry and email, remove the transient keys, and clean the
     URL with `history.replaceState` so a reload does not replay a spent code.
     Cognito's `error`/`error_description` parameters surface as an error state;
   - `access_token()` returning `None` when unconfigured, absent, or past
     `expires_at`; `sign_out()` clearing storage and going to `/logout`;
   - a small `decode_jwt_claims` helper — base64url-decode the payload segment,
     `serde_json` it into `{ email: Option<String> }`.

3. **`crates/app/src/api.rs`** — attach `Authorization: Bearer <token>` when
   `auth::access_token()` returns one, and give 401 a distinguishable error so
   the UI can offer to sign in again rather than printing a status code. The
   `url()` helper and the existing error mapping stay as they are.

4. **`crates/app/src/app.rs`** — an auth-state signal (`Loading`, `SignedOut`,
   `SignedIn`, `Disabled`, `Error`) provided through context, settled once by
   `complete_sign_in()` at mount. The header gains a sign-in / sign-out control,
   rendered only when configured, showing the email when signed in.

   **The load-bearing detail:** `HomePage`'s `LocalResource` must not fire before
   the callback exchange has settled, or the first render after a sign-in fetches
   without a token, renders the 401, and never retries. Make the resource's
   source closure read the auth signal so it re-runs when the state settles.

5. **`style/main.css`** — style the new control alongside the existing header
   rules. Plain CSS, no framework, per `docs/design/frontend.md`.

6. **`justfile`** — a `dev-web-auth` recipe resolving both values through the
   existing `_ssm` helper and exporting them around `trunk serve`; and the same
   two variables added beside `API_BASE_URL` in `deploy-web`, whose comment about
   nothing in `crates/` reading them stops being true.

7. **`docs/design/frontend.md`** — a Data-fetching/authentication paragraph, the
   compile-time variables, and the replacement of the constraint that says the
   SPA obtains no token. Draft it and have the user confirm before it lands:
   Design Documents are overwritten, and `docs/README.md` reserves that for a
   human.

8. **`docs/design/deployment.md` and `docs/design/index.md`** — same treatment.
   The "Configuring the SPA" table gains two rows, the note at the top of the
   file and the "SPA sends no `Authorization` header yet" constraint both go, and
   the index gains the new Decision Record row.

9. **`DR-0010`** — the sign-in decision: hosted UI with PKCE implemented by hand,
   access token in `sessionStorage`, no refresh token, no automatic redirect on
   401, sign-in disabled when unconfigured. Alternatives to record and reject:
   keeping the refresh token, tokens in memory only, `localStorage`, and an
   off-the-shelf auth library. This is the record that makes the Work Log safe to
   delete.

## Verification

Steps 1–6 are checkable here and now:

```sh
just fmt-check && just check && just lint && just test && just build
```

**Unconfigured path — no credentials needed.** `just dev-api` in one shell,
`trunk serve` in another, then `http://localhost:8080`: the greeting renders, no
sign-in control appears, and the network panel shows the `/api/greeting` request
carrying no `Authorization` header. This is the regression that matters most —
the zero-configuration development path must not have moved.

**Configured path — needs the two values.** Either `aws login` and then
`just dev-web-auth`, or export `COGNITO_CLIENT_ID` and
`COGNITO_HOSTED_UI_DOMAIN` by hand if the values are known, and run
`trunk serve`. Then: the sign-in control appears; it lands on the hosted UI and
on Google; the return to `http://localhost:8080/` clears `?code=` from the
address bar; the email shows in the header; the greeting renders; the request
carries `Authorization: Bearer …`; a reload keeps the session; sign-out clears it
and returns to the signed-out state.

Note what this does *not* prove: locally the request goes to the axum server,
which ignores the header. It verifies the client half only.

**The authorizer half, without deploying.** Copy the access token out of
`sessionStorage` after signing in locally and call the deployed API directly:

```sh
endpoint="$(just _ssm api/api_endpoint)"
curl -i -H "Authorization: Bearer ${TOKEN}" "${endpoint%/}/api/greeting"
```

200 with the greeting JSON is the end-to-end proof that the token the SPA obtains
is one API Gateway accepts — the gap closed, without an upload. The same call
without the header still returning 401 confirms the authorizer is doing its job.
This needs AWS credentials for the SSM lookup only; the endpoint can be pasted
instead.

## Assumptions

- No Terraform change is needed. The app client, its callback and logout URLs,
  its scopes, and the authorizer's audience are all already correct for this
  flow; nothing under `infra/` is touched.
- `crates/server` is not touched either. API Gateway is the enforcement point,
  and adding JWT validation to the service would duplicate it and break the
  single-origin local setup. Worth revisiting only if the API ever gains a
  second caller.
- The Google-issued email is present in the id token; the header falls back to a
  generic signed-in label when it is not.
