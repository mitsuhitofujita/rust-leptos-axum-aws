//! Calls to the axum API.
//!
//! Paths are always absolute and are joined to [`API_BASE_URL`], which is empty
//! in a development build: `Trunk.toml` proxies `/api` to the API server, so the
//! browser sees a single origin.

use gloo_net::http::Request;
use shared::Greeting;

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

pub async fn fetch_greeting() -> Result<Greeting, String> {
    let response = Request::get(&url("/api/greeting"))
        .send()
        .await
        .map_err(|err| format!("request failed: {err}"))?;

    if !response.ok() {
        return Err(format!("unexpected status: {}", response.status()));
    }

    response
        .json::<Greeting>()
        .await
        .map_err(|err| format!("could not decode the response: {err}"))
}
