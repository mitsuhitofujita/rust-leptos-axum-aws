//! Calls to the axum API.
//!
//! Requests are made against a relative path. In development `Trunk.toml`
//! proxies `/api` to the API server, so the browser sees a single origin.

use gloo_net::http::Request;
use shared::Greeting;

pub async fn fetch_greeting() -> Result<Greeting, String> {
    let response = Request::get("/api/greeting")
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
