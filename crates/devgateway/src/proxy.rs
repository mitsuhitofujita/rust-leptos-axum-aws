//! The forwarding leg: what the Lambda Web Adapter does once the `AuthContext`
//! is on.
//!
//! Plain HTTP to loopback, so there is no TLS here and no TLS dependency in the
//! image. The body is streamed rather than buffered, because nothing in this
//! process has any reason to look at it.

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::uri::{PathAndQuery, Uri};
use axum::http::{Request, StatusCode};
use axum::response::Response;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;

/// Headers that describe one hop and must not be copied onto the next.
const HOP_BY_HOP: [&str; 8] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

pub type Forwarder = Client<HttpConnector, Body>;

pub fn forwarder() -> Forwarder {
    Client::builder(TokioExecutor::new()).build_http()
}

pub async fn forward(
    forwarder: &Forwarder,
    upstream: &str,
    mut parts: Parts,
    body: Body,
) -> Response {
    let path_and_query = parts
        .uri
        .path_and_query()
        .map_or_else(|| parts.uri.path(), PathAndQuery::as_str);

    let uri: Uri = match format!("{upstream}{path_and_query}").parse() {
        Ok(uri) => uri,
        Err(error) => return unreachable_upstream(format!("{upstream} is not a URI: {error}")),
    };

    for header in HOP_BY_HOP {
        parts.headers.remove(header);
    }
    // hyper writes the authority it is connecting to; a Host left over from the
    // caller would name the adapter.
    parts.headers.remove("host");

    parts.uri = uri;

    match forwarder.request(Request::from_parts(parts, body)).await {
        Ok(response) => response.map(Body::new),
        // The one failure a developer sees often: the service is not running.
        Err(error) => unreachable_upstream(format!("{upstream} did not answer: {error}")),
    }
}

/// An integration that does not answer is a 500 at the HTTP API too, so the
/// status is right; the body is not API Gateway's, because a developer reading
/// it is better served by the reason than by `{"message":"Internal Server
/// Error"}`.
fn unreachable_upstream(reason: String) -> Response {
    println!("devgateway: {reason}");

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"message":"Internal Server Error"}"#.to_owned(),
        ))
        .expect("a valid response")
}
