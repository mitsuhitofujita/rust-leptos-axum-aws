//! Who the request is from.
//!
//! The service never validates a token. API Gateway's JWT authorizer has
//! already done it, and it is the only enforcement point there is (DR-0010).
//! What reaches the function is the authorizer's conclusion: the claims, inside
//! the request context of the event.
//!
//! `crates/server` is an ordinary axum binary and never sees that event —
//! the Lambda Web Adapter turns it into an HTTP request and puts the request
//! context back on as the `x-amzn-request-context` header, serialised as JSON.
//! So the subject arrives as a header, and reading it is all this does.
//!
//! Nothing here is a security boundary. A header can be forged by anyone who
//! can reach the service directly; what makes that irrelevant is that nothing
//! can — API Gateway is the only route to the function, and it overwrites this
//! header on every request.

use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::HeaderMap;
use axum::http::request::Parts;
use serde::Deserialize;

/// The header the Lambda Web Adapter carries the API Gateway request context in.
const REQUEST_CONTEXT: &str = "x-amzn-request-context";

/// Who owns the items a request reads and writes when nothing says otherwise.
///
/// There is no header under `just dev-api`: no API Gateway, no authorizer, and
/// no token — the SPA is not even configured to obtain one unless
/// `just dev-web-auth` is what started it. An unset value meaning something
/// workable rather than something broken is the same choice DR-0008 makes for
/// the frontend's configuration, and this is its equivalent here.
///
/// It is a partition key like any other, so development data sits beside real
/// data without mixing with it. In a deployed function this is unreachable:
/// every request arrives through the authorizer, which is what puts the header
/// there.
const DEVELOPMENT_OWNER: &str = "development";

/// The Cognito subject of whoever made the request.
///
/// A `sub` rather than an email address: it is what the token asserts, and
/// unlike an address it does not change (DR-0015).
pub struct Owner(pub String);

impl<S: Send + Sync> FromRequestParts<S> for Owner {
    /// This never rejects. A request with no request context is a request from
    /// a developer's own machine, and [`DEVELOPMENT_OWNER`] is who that is.
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            subject(&parts.headers).unwrap_or_else(|| DEVELOPMENT_OWNER.to_owned()),
        ))
    }
}

fn subject(headers: &HeaderMap) -> Option<String> {
    let context = headers.get(REQUEST_CONTEXT)?.to_str().ok()?;
    let context: RequestContext = serde_json::from_str(context).ok()?;
    context.authorizer?.jwt?.claims.remove("sub")
}

/// As much of the API Gateway HTTP API request context as the subject needs.
///
/// The adapter serialises `lambda_http`'s own `RequestContext`, which is
/// `#[serde(untagged)]`, so an HTTP API's context is the whole of the JSON
/// rather than a variant inside it. Everything not named here is ignored, which
/// is nearly all of it.
#[derive(Deserialize)]
struct RequestContext {
    authorizer: Option<Authorizer>,
}

#[derive(Deserialize)]
struct Authorizer {
    jwt: Option<Jwt>,
}

#[derive(Deserialize)]
struct Jwt {
    claims: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_subject_from_the_request_context() {
        let mut headers = HeaderMap::new();
        headers.insert(
            REQUEST_CONTEXT,
            r#"{"accountId":"1","authorizer":{"jwt":{"claims":{"sub":"abc-123","client_id":"x"},"scopes":null}}}"#
                .parse()
                .unwrap(),
        );

        assert_eq!(subject(&headers).as_deref(), Some("abc-123"));
    }

    #[test]
    fn has_no_subject_without_the_header() {
        assert_eq!(subject(&HeaderMap::new()), None);
    }

    /// `/health` is routed outside the authorizer, so a context with no
    /// authorizer at all is a shape that really arrives.
    #[test]
    fn has_no_subject_when_the_context_carries_no_authorizer() {
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_CONTEXT, r#"{"accountId":"1"}"#.parse().unwrap());

        assert_eq!(subject(&headers), None);
    }
}
