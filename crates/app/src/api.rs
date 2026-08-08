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

use gloo_net::http::Request;
use shared::Greeting;

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

/// Starts a request to an absolute API path, carrying the access token when the
/// tab holds one.
///
/// No token is not an error: an unconfigured build has none, and the local API
/// checks nothing, so the call goes out bare and is answered.
fn get(path: &str) -> gloo_net::http::RequestBuilder {
    let request = Request::get(&url(path));
    match auth::access_token() {
        Some(token) => request.header("Authorization", &format!("Bearer {token}")),
        None => request,
    }
}

pub async fn fetch_greeting() -> Result<Greeting, ApiError> {
    let response = get("/api/greeting")
        .send()
        .await
        .map_err(|err| ApiError::Other(format!("request failed: {err}")))?;

    if response.status() == 401 {
        return Err(ApiError::Unauthorized);
    }

    if !response.ok() {
        return Err(ApiError::Other(format!(
            "unexpected status: {}",
            response.status()
        )));
    }

    response
        .json::<Greeting>()
        .await
        .map_err(|err| ApiError::Other(format!("could not decode the response: {err}")))
}
