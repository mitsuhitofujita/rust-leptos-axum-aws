//! Sign-in against the Cognito hosted UI.
//!
//! Authorization Code Flow with PKCE, written out by hand: the flow is two URLs
//! and one form POST, and every library that wraps it is larger than the code it
//! would replace (DR-0010).
//!
//! The whole module is inert unless both compile-time variables below are set,
//! which is what keeps `trunk serve` needing no configuration — the same shape
//! [`crate::api`] gives an unset `API_BASE_URL` (DR-0008).

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use gloo_net::http::Request;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use wasm_bindgen::JsValue;
use web_sys::{Storage, UrlSearchParams, Window};

/// The public app client, baked into the bundle at compile time.
///
/// Published by the `identity` layer as `/<project>/identity/app_client_id`, and
/// resolved from there by `just deploy-web` and `just dev-web-auth`. It is not a
/// secret: a WASM bundle the browser downloads cannot keep one, which is why the
/// client has no secret and PKCE stands in for it.
const CLIENT_ID: &str = match option_env!("COGNITO_CLIENT_ID") {
    Some(id) => id,
    None => "",
};

/// The hosted UI's hostname, baked in the same way.
///
/// Published as `/<project>/identity/hosted_ui_domain`, in the bare form
/// `<prefix>.auth.<region>.amazoncognito.com` — no scheme, which [`hosted_ui`]
/// supplies.
const HOSTED_UI_DOMAIN: &str = match option_env!("COGNITO_HOSTED_UI_DOMAIN") {
    Some(domain) => domain,
    None => "",
};

/// The scopes the app client allows. `email` is what puts an address in the id
/// token, which is the only thing the id token is read for.
const SCOPES: &str = "openid email profile";

/// The bearer token sent to the API.
const KEY_ACCESS_TOKEN: &str = "auth.access_token";
/// `Date.now()` at the exchange plus `expires_in`, in milliseconds.
const KEY_EXPIRES_AT: &str = "auth.expires_at";
/// The address shown in the header, or absent when the id token carried none.
const KEY_EMAIL: &str = "auth.email";
/// Live for the duration of one redirect only, removed on return.
const KEY_VERIFIER: &str = "auth.pkce_verifier";
const KEY_STATE: &str = "auth.state";

/// What the header renders and what [`crate::api`] can expect of a request.
///
/// `Disabled` is not a failure: it is an unconfigured build, where no control is
/// shown and the local API — which checks nothing — keeps answering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
    /// Before the callback exchange has settled, at mount only.
    Loading,
    /// Neither compile-time variable was set.
    Disabled,
    SignedOut,
    SignedIn {
        email: Option<String>,
    },
    /// The hosted UI or the token endpoint refused. Recoverable by signing in
    /// again, so the control stays offered alongside the message.
    Error(String),
}

/// The `email` claim, the only part of the id token that is read.
///
/// This decode is for display. The token is not verified here and nothing is
/// authorised by it — API Gateway's authorizer is the security boundary.
#[derive(Deserialize)]
struct IdTokenClaims {
    email: Option<String>,
}

/// Cognito's `/oauth2/token` response. The refresh token is deliberately not
/// among these fields: nothing keeps one (DR-0010).
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: Option<String>,
    /// Seconds. Cognito sends a number, not a string.
    expires_in: f64,
}

/// Whether this build knows where to sign in.
///
/// Either variable empty means it does not, and every entry point below turns
/// into a no-op rather than a half-configured redirect.
pub fn is_configured() -> bool {
    !CLIENT_ID.is_empty() && !HOSTED_UI_DOMAIN.is_empty()
}

/// Sends the browser to the hosted UI.
///
/// Generates the PKCE verifier and the `state`, stores both for the return trip,
/// and navigates. Nothing after the assignment to `location.href` runs in
/// practice, so this returns only to report a failure before that point.
pub fn begin_sign_in() -> Result<(), String> {
    if !is_configured() {
        return Err("sign-in is not configured in this build".to_owned());
    }

    // 32 bytes is RFC 7636's ceiling for the verifier's entropy and 16 is ample
    // for a value whose only job is to match itself on the way back.
    let verifier = random_base64url(32)?;
    let state = random_base64url(16)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let store = storage()?;
    store.set_item(KEY_VERIFIER, &verifier).map_err(js_error)?;
    store.set_item(KEY_STATE, &state).map_err(js_error)?;

    let query = UrlSearchParams::new().map_err(js_error)?;
    query.append("client_id", CLIENT_ID);
    query.append("response_type", "code");
    query.append("scope", SCOPES);
    query.append("redirect_uri", &redirect_uri()?);
    query.append("state", &state);
    query.append("code_challenge_method", "S256");
    query.append("code_challenge", &challenge);

    let url = format!("{}?{}", hosted_ui("/oauth2/authorize"), params(&query));
    window()?.location().set_href(&url).map_err(js_error)
}

/// Settles the auth state once, at mount.
///
/// Three cases: a `code` in the query means the visitor is coming back from the
/// hosted UI and the exchange happens here; an `error` means the hosted UI
/// refused; anything else means an ordinary load, which reports whatever session
/// the tab still holds.
pub async fn complete_sign_in() -> AuthState {
    if !is_configured() {
        return AuthState::Disabled;
    }

    match settle().await {
        Ok(state) => state,
        Err(message) => AuthState::Error(message),
    }
}

async fn settle() -> Result<AuthState, String> {
    let search = window()?.location().search().map_err(js_error)?;
    let query = UrlSearchParams::new_with_str(&search).map_err(js_error)?;

    if let Some(error) = query.get("error") {
        // The redirect is over either way, so the transient keys and the query
        // string go regardless of the outcome.
        forget_transient();
        clean_url()?;
        return Ok(AuthState::Error(
            query.get("error_description").unwrap_or(error),
        ));
    }

    let Some(code) = query.get("code") else {
        return Ok(restore_session());
    };

    let returned_state = query.get("state");
    let expected_state = read(KEY_STATE);
    let verifier = read(KEY_VERIFIER);
    forget_transient();
    clean_url()?;

    // A `code` arriving without the pair this browser stored is not a session to
    // adopt: either the tab never started this sign-in, or the response was not
    // the one it asked for.
    if returned_state.is_none() || returned_state != expected_state {
        return Ok(AuthState::Error(
            "the sign-in response did not match this browser's request".to_owned(),
        ));
    }

    let verifier = verifier.ok_or("this browser has no PKCE verifier for that sign-in")?;
    exchange(&code, &verifier).await
}

/// Trades the authorization code for tokens and stores the session.
///
/// No client secret: the app client is public, and the verifier is what proves
/// this is the browser that asked for the code.
async fn exchange(code: &str, verifier: &str) -> Result<AuthState, String> {
    let form = UrlSearchParams::new().map_err(js_error)?;
    form.append("grant_type", "authorization_code");
    form.append("client_id", CLIENT_ID);
    form.append("code", code);
    // Must be byte-identical to the one sent to /oauth2/authorize; both are
    // computed by `redirect_uri`, so they cannot drift.
    form.append("redirect_uri", &redirect_uri()?);
    form.append("code_verifier", verifier);

    let response = Request::post(&hosted_ui("/oauth2/token"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(params(&form))
        .map_err(|err| format!("could not build the token request: {err}"))?
        .send()
        .await
        .map_err(|err| format!("the token request failed: {err}"))?;

    if !response.ok() {
        return Err(format!("the token endpoint answered {}", response.status()));
    }

    let token = response
        .json::<TokenResponse>()
        .await
        .map_err(|err| format!("could not decode the token response: {err}"))?;

    // The access token is what the API receives: the authorizer's audience is
    // the app client id, which a Cognito access token carries as `client_id`.
    // The id token is read once, for the address, and then dropped.
    let email = token
        .id_token
        .as_deref()
        .and_then(decode_jwt_claims)
        .and_then(|claims| claims.email);

    let store = storage()?;
    store
        .set_item(KEY_ACCESS_TOKEN, &token.access_token)
        .map_err(js_error)?;
    store
        .set_item(
            KEY_EXPIRES_AT,
            &(js_sys::Date::now() + token.expires_in * 1000.0).to_string(),
        )
        .map_err(js_error)?;
    match &email {
        Some(email) => store.set_item(KEY_EMAIL, email).map_err(js_error)?,
        None => store.remove_item(KEY_EMAIL).map_err(js_error)?,
    }

    Ok(AuthState::SignedIn { email })
}

/// The token to put in an `Authorization` header, if there is a usable one.
///
/// `None` covers every reason there might not be: an unconfigured build, a tab
/// that never signed in, and a session past its expiry — which is dropped here,
/// so an expired token is never sent in the first place.
pub fn access_token() -> Option<String> {
    if !is_configured() {
        return None;
    }

    let expires_at = read(KEY_EXPIRES_AT)?.parse::<f64>().ok()?;
    if js_sys::Date::now() >= expires_at {
        forget_session();
        return None;
    }

    read(KEY_ACCESS_TOKEN)
}

/// Drops the session and sends the browser to the hosted UI's logout endpoint,
/// which clears Cognito's own cookie and returns here signed out.
pub fn sign_out() -> Result<(), String> {
    forget_session();
    if !is_configured() {
        return Ok(());
    }

    let query = UrlSearchParams::new().map_err(js_error)?;
    query.append("client_id", CLIENT_ID);
    // `logout_uri`, not `redirect_uri`: this is the logout endpoint's own
    // parameter name, and the value is registered as a logout URL on the client.
    query.append("logout_uri", &redirect_uri()?);

    let url = format!("{}?{}", hosted_ui("/logout"), params(&query));
    window()?.location().set_href(&url).map_err(js_error)
}

/// Drops the local session without leaving the page.
///
/// What an unexpected 401 does. Redirecting to the hosted UI instead would loop
/// for as long as the API returns 401 for any reason other than the token, so
/// the visitor is asked rather than sent (DR-0010).
pub fn forget_session() {
    let Ok(store) = storage() else {
        return;
    };
    for key in [KEY_ACCESS_TOKEN, KEY_EXPIRES_AT, KEY_EMAIL] {
        let _ = store.remove_item(key);
    }
}

fn forget_transient() {
    let Ok(store) = storage() else {
        return;
    };
    for key in [KEY_VERIFIER, KEY_STATE] {
        let _ = store.remove_item(key);
    }
}

/// Reports the session this tab already holds, on an ordinary load.
fn restore_session() -> AuthState {
    match access_token() {
        Some(_) => AuthState::SignedIn {
            email: read(KEY_EMAIL),
        },
        None => AuthState::SignedOut,
    }
}

/// Where the hosted UI sends the browser back to.
///
/// Computed rather than configured: it is `https://<cloudfront>/` in a deployed
/// build and `http://localhost:8080/` under `trunk serve`, and both are already
/// registered on the app client. One less build variable, and no way for the
/// value sent to `/oauth2/authorize` and the one sent to `/oauth2/token` to
/// disagree.
fn redirect_uri() -> Result<String, String> {
    let origin = window()?.location().origin().map_err(js_error)?;
    Ok(format!("{}/", origin.trim_end_matches('/')))
}

/// Joins a hosted-UI path onto [`HOSTED_UI_DOMAIN`], supplying the scheme SSM's
/// bare hostname does not carry, and tolerating one that already has it.
fn hosted_ui(path: &str) -> String {
    let domain = HOSTED_UI_DOMAIN.trim_end_matches('/');
    if domain.starts_with("https://") || domain.starts_with("http://") {
        format!("{domain}{path}")
    } else {
        format!("https://{domain}{path}")
    }
}

/// Percent-encodes a parameter set, for a query string or a form body.
fn params(query: &UrlSearchParams) -> String {
    String::from(query.to_string())
}

/// Strips the query string from the address bar, so a reload cannot replay a
/// spent authorization code — Cognito rejects a second use, and the rejection
/// would surface as an error on a page that had already signed in.
fn clean_url() -> Result<(), String> {
    let window = window()?;
    let path = window.location().pathname().map_err(js_error)?;
    window
        .history()
        .map_err(js_error)?
        .replace_state_with_url(&JsValue::NULL, "", Some(&path))
        .map_err(js_error)
}

/// Reads the payload segment of a JWT.
///
/// Display only, as [`IdTokenClaims`] records: the signature is not checked, and
/// nothing downstream trusts the result with anything but a label.
fn decode_jwt_claims(token: &str) -> Option<IdTokenClaims> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn random_base64url(bytes: usize) -> Result<String, String> {
    let mut buffer = vec![0u8; bytes];
    window()?
        .crypto()
        .map_err(js_error)?
        .get_random_values_with_u8_array(&mut buffer)
        .map_err(js_error)?;
    Ok(URL_SAFE_NO_PAD.encode(buffer))
}

fn read(key: &str) -> Option<String> {
    storage().ok()?.get_item(key).ok().flatten()
}

fn window() -> Result<Window, String> {
    web_sys::window().ok_or_else(|| "no browser window".to_owned())
}

/// `sessionStorage`, not `localStorage`: the session ends with the tab, which is
/// the shortest lifetime that still survives a reload (DR-0010). It is absent
/// rather than empty when a browser blocks storage, which every caller treats as
/// "not signed in".
fn storage() -> Result<Storage, String> {
    window()?
        .session_storage()
        .map_err(js_error)?
        .ok_or_else(|| "sessionStorage is unavailable".to_owned())
}

/// Browser exceptions carry no `Display`, and the flow's errors are rendered.
fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| format!("the browser refused the call: {value:?}"))
}
