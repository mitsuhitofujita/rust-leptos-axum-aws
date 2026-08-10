//! Calls to the axum API.
//!
//! Paths are always absolute and are joined to [`API_BASE_URL`], which is empty
//! in a development build: `Trunk.toml` proxies `/api` to the API server, so the
//! browser sees a single origin.
//!
//! Every call carries the Cognito access token from [`crate::auth`] when there
//! is one. In production there has to be: API Gateway puts a JWT authorizer in
//! front of `/api/{proxy+}`. Locally there is not, and the axum server behind
//! the proxy checks nothing, so the same code path serves both.

use std::fmt;

use gloo_net::http::{RequestBuilder, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use shared::{ActionType, Dashboard, NewActionType};

use crate::auth;

/// The origin the API answers on, baked into the bundle at compile time.
///
/// Unset in development, where the trunk proxy makes `/api` same-origin. For a
/// deployed build `just deploy-web` reads the endpoint from SSM and sets it,
/// which makes the same call cross-origin against API Gateway — the CORS
/// configuration in `infra/api` is what lets the browser make it.
///
/// Reading it here rather than writing a hostname into the source is what keeps
/// the frontend free of one, and because the value lands inside a
/// content-hashed bundle, changing it invalidates its own cache entry.
const API_BASE_URL: &str = match option_env!("API_BASE_URL") {
    Some(base) => base,
    None => "",
};

/// Joins an absolute API path onto [`API_BASE_URL`].
///
/// The base is trimmed because API Gateway's `$default` stage publishes its
/// invoke URL with a trailing slash, which would otherwise double the separator.
fn url(path: &str) -> String {
    format!("{}{path}", API_BASE_URL.trim_end_matches('/'))
}

/// Why a call did not produce a value.
///
/// 401 is separated out because it is the one failure the visitor can act on:
/// API Gateway's authorizer rejected the token, or there was none to send. The
/// UI offers a fresh sign-in instead of rendering a status code — it does not
/// redirect on its own, which would loop for any 401 the token cannot fix
/// (DR-0010).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiError {
    Unauthorized,
    Other(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized => formatter.write_str("this request was not authorized"),
            Self::Other(message) => formatter.write_str(message),
        }
    }
}

/// Attaches the access token when the tab holds one.
///
/// No token is not an error: an unconfigured build has none, and the local API
/// checks nothing, so the call goes out bare and is answered.
fn authorized(request: RequestBuilder) -> RequestBuilder {
    match auth::access_token() {
        Some(token) => request.header("Authorization", &format!("Bearer {token}")),
        None => request,
    }
}

/// Sends a GET and decodes its body.
async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    let response = authorized(gloo_net::http::Request::get(&url(path)))
        .send()
        .await
        .map_err(|err| ApiError::Other(format!("request failed: {err}")))?;

    decode(response).await
}

/// Sends a JSON body and decodes what comes back.
async fn post_json<B: Serialize, T: DeserializeOwned>(path: &str, body: &B) -> Result<T, ApiError> {
    let request = authorized(gloo_net::http::Request::post(&url(path)))
        .json(body)
        .map_err(|err| ApiError::Other(format!("could not encode the request: {err}")))?;

    let response = request
        .send()
        .await
        .map_err(|err| ApiError::Other(format!("request failed: {err}")))?;

    decode(response).await
}

/// Turns a response into a value or an [`ApiError`].
///
/// One place separates 401 from every other failure, so every endpoint below
/// inherits that treatment rather than restating it.
///
/// A rejected request answers with its reason in the body, and that reason is
/// what the visitor needs — a screen reporting `400` where it could report
/// `unit is required` is reporting the wrong thing. The status stands in only
/// when there is nothing to say.
async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, ApiError> {
    if response.status() == 401 {
        return Err(ApiError::Unauthorized);
    }

    if !response.ok() {
        let status = response.status();
        let reason = response.text().await.unwrap_or_default();
        return Err(ApiError::Other(if reason.trim().is_empty() {
            format!("unexpected status: {status}")
        } else {
            reason
        }));
    }

    response
        .json::<T>()
        .await
        .map_err(|err| ApiError::Other(format!("could not decode the response: {err}")))
}

/// The ten-day summary and the ten most recent records, in one response.
pub async fn fetch_dashboard() -> Result<Dashboard, ApiError> {
    get_json("/api/dashboard").await
}

/// Every action type this account has registered, oldest first.
///
/// The order is the store's: the identifier is a ULID and the query returns the
/// partition in key order, so registration order comes for free. No screen has
/// asked for another one.
pub async fn fetch_action_types() -> Result<Vec<ActionType>, ApiError> {
    get_json("/api/action-types").await
}

/// Registers an action type, and answers with the stored one — including the
/// identifier the service assigned it.
pub async fn create_action_type(new: &NewActionType) -> Result<ActionType, ApiError> {
    post_json("/api/action-types", new).await
}
