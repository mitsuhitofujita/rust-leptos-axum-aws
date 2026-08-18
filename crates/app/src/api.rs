//! Calls to the axum API.
//!
//! Paths are always absolute and are joined to [`API_BASE_URL`], which is empty
//! in a development build: `Trunk.toml` proxies `/api` to the API server, so the
//! browser sees a single origin.
//!
//! Every call carries the Cognito access token from [`crate::auth`] when there
//! is one. In production there has to be: `crates/server` verifies it itself
//! against the pool (DR-0028). Locally there is not, and the axum server
//! behind the proxy checks nothing under `Auth::Mock`, so the same code path
//! serves both.

use std::fmt;

use gloo_net::http::{RequestBuilder, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use shared::{
    ActionRecord, ActionType, Dashboard, NewActionRecord, NewActionType, UpdateActionRecord,
};

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
    json_body(gloo_net::http::Request::post(&url(path)), body).await
}

/// Sends a JSON body as a `PUT` and decodes what comes back.
async fn put_json<B: Serialize, T: DeserializeOwned>(path: &str, body: &B) -> Result<T, ApiError> {
    json_body(gloo_net::http::Request::put(&url(path)), body).await
}

async fn json_body<B: Serialize, T: DeserializeOwned>(
    request: RequestBuilder,
    body: &B,
) -> Result<T, ApiError> {
    let request = authorized(request)
        .json(body)
        .map_err(|err| ApiError::Other(format!("could not encode the request: {err}")))?;

    let response = request
        .send()
        .await
        .map_err(|err| ApiError::Other(format!("request failed: {err}")))?;

    decode(response).await
}

/// Sends a `DELETE` with no body. There is nothing to decode from the `204`
/// this answers with on success, unlike `decode`'s callers.
async fn delete(path: &str) -> Result<(), ApiError> {
    let response = authorized(gloo_net::http::Request::delete(&url(path)))
        .send()
        .await
        .map_err(|err| ApiError::Other(format!("request failed: {err}")))?;

    checked(response).await.map(|_| ())
}

/// Separates 401 from every other failure, so every endpoint below inherits
/// that treatment rather than restating it.
///
/// A rejected request answers with its reason in the body, and that reason is
/// what the visitor needs — a screen reporting `400` where it could report
/// `unit is required` is reporting the wrong thing. The status stands in only
/// when there is nothing to say.
async fn checked(response: Response) -> Result<Response, ApiError> {
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

    Ok(response)
}

/// Turns a response into a value or an [`ApiError`].
async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, ApiError> {
    checked(response)
        .await?
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

/// One action type by id, for the edit screen to fill its form from.
pub async fn fetch_action_type(id: &str) -> Result<ActionType, ApiError> {
    get_json(&format!("/api/action-types/{id}")).await
}

/// Saves changes to an action type, and answers with it as stored.
///
/// A record that already copied this type's prior name, unit or icon keeps its
/// copy — DR-0016. Nothing about that is this call's concern; it changes only
/// the type.
pub async fn update_action_type(id: &str, new: &NewActionType) -> Result<ActionType, ApiError> {
    put_json(&format!("/api/action-types/{id}"), new).await
}

/// Removes an action type. Idempotent, like the `DeleteItem` it becomes: a
/// second call for the same id is not an error.
pub async fn delete_action_type(id: &str) -> Result<(), ApiError> {
    delete(&format!("/api/action-types/{id}")).await
}

/// Every action record this account has, newest first.
pub async fn fetch_action_records() -> Result<Vec<ActionRecord>, ApiError> {
    get_json("/api/actions").await
}

/// Records one action against a registered type, and answers with it as
/// stored — including the identifier the service assigned it and the type's
/// display attributes as it copied them (DR-0016).
pub async fn create_action_record(new: &NewActionRecord) -> Result<ActionRecord, ApiError> {
    post_json("/api/actions", new).await
}

/// One action record by id, for the edit screen to fill its form from.
pub async fn fetch_action_record(id: &str) -> Result<ActionRecord, ApiError> {
    get_json(&format!("/api/actions/{id}")).await
}

/// Changes the recorded value, and answers with the record as stored. The
/// type and its copied display attributes are fixed once a record is
/// created — DR-0016 — so there is nothing else to send.
pub async fn update_action_record(
    id: &str,
    new: &UpdateActionRecord,
) -> Result<ActionRecord, ApiError> {
    put_json(&format!("/api/actions/{id}"), new).await
}

/// Removes an action record. Idempotent, like `delete_action_type`.
pub async fn delete_action_record(id: &str) -> Result<(), ApiError> {
    delete(&format!("/api/actions/{id}")).await
}
