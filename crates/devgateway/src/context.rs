//! The request context the Lambda Web Adapter forwards, written the way API
//! Gateway writes it.
//!
//! `crates/server` never sees a Lambda event. The adapter turns the invocation
//! into an HTTP request and puts the event's request context back on as the
//! `x-amzn-request-context` header, serialised as JSON — DR-0017. This module
//! is the half of the stand-in that produces that header.
//!
//! Two properties of the real one are load-bearing and are the reason this is
//! written by hand rather than approximated:
//!
//! - **Every claim is a string.** Payload format 2.0 stringifies whatever the
//!   token held, and `crates/server/src/identity.rs` deserialises the claims as
//!   `HashMap<String, String>`. One non-string value fails the decode of the
//!   whole context, the subject comes back as `None`, and the request is
//!   attributed to the development owner instead of being refused — silently, in
//!   every environment. Nothing but this observes it.
//! - **An unauthenticated route carries no `authorizer` member at all**, rather
//!   than an empty one. `/health` is routed outside the authorizer, and that is
//!   the shape it produces.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::request::Parts;
use serde_json::{Map, Value, json};

/// The context for a route that sits outside the authorizer.
pub fn unauthenticated(parts: &Parts, route_key: &str) -> String {
    base(parts, route_key).to_string()
}

/// The context for a route behind the authorizer, carrying its conclusion.
pub fn authorized(parts: &Parts, route_key: &str, claims: &Map<String, Value>) -> String {
    let mut context = base(parts, route_key);

    context["authorizer"] = json!({
        "jwt": {
            "claims": stringify(claims),
            // The deployed authorizer declares no scopes, so a Cognito access
            // token's `scope` claim stays in the claims and this stays null.
            "scopes": Value::Null,
        }
    });

    context.to_string()
}

/// Claims as API Gateway hands them on: every value a string.
///
/// A list claim — `cognito:groups` is the one that really arrives — is rendered
/// bracketed and space-separated rather than as JSON, which is the detail a
/// stand-in that reached for `serde_json::to_string` would get wrong.
fn stringify(claims: &Map<String, Value>) -> Map<String, Value> {
    claims
        .iter()
        .map(|(name, value)| (name.clone(), Value::String(flatten(value))))
        .collect()
}

fn flatten(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(flatten).collect();
            format!("[{}]", items.join(" "))
        }
        other => other.to_string(),
    }
}

/// As much of an HTTP API's payload-2.0 request context as is worth writing.
///
/// The service reads `authorizer.jwt.claims` and ignores the rest, but the rest
/// is what makes this recognisable to anyone who has read a real one, and what
/// a future reader of the header will compare against.
fn base(parts: &Parts, route_key: &str) -> Value {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    json!({
        "accountId": "local",
        "apiId": "devgateway",
        "domainName": "localhost",
        "domainPrefix": "localhost",
        "http": {
            "method": parts.method.as_str(),
            "path": parts.uri.path(),
            "protocol": "HTTP/1.1",
            "sourceIp": "127.0.0.1",
            "userAgent": header(parts, "user-agent"),
        },
        "requestId": format!("{:x}", now.as_nanos()),
        "routeKey": route_key,
        "stage": "$default",
        "timeEpoch": now.as_millis() as u64,
    })
}

fn header(parts: &Parts, name: &str) -> String {
    parts
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::http::Request;
    use serde::Deserialize;

    use super::*;

    fn parts() -> Parts {
        Request::get("/api/action-types")
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    /// The shape `crates/server/src/identity.rs` deserialises. Kept here as a
    /// copy on purpose: this test fails for exactly the reason the service
    /// would, rather than for a reason of its own.
    #[derive(Deserialize)]
    struct ServiceView {
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

    fn claims_of(context: &str) -> HashMap<String, String> {
        serde_json::from_str::<ServiceView>(context)
            .expect("the service must be able to decode the context")
            .authorizer
            .expect("an authorized route carries an authorizer")
            .jwt
            .expect("a JWT authorizer carries a jwt member")
            .claims
    }

    /// The silent failure this whole stand-in exists to make visible: a token
    /// carrying anything but strings must still decode into
    /// `HashMap<String, String>` on the other side.
    #[test]
    fn every_claim_reaches_the_service_as_a_string() {
        let claims = serde_json::from_str(
            r#"{
                "sub": "abc-123",
                "exp": 1754870400,
                "email_verified": true,
                "cognito:groups": ["admins", "beta"]
            }"#,
        )
        .unwrap();

        let claims = claims_of(&authorized(&parts(), "GET /api/{proxy+}", &claims));

        assert_eq!(claims.get("sub").map(String::as_str), Some("abc-123"));
        assert_eq!(claims.get("exp").map(String::as_str), Some("1754870400"));
        assert_eq!(
            claims.get("email_verified").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            claims.get("cognito:groups").map(String::as_str),
            Some("[admins beta]")
        );
    }

    /// `/health` is routed outside the authorizer, and `identity.rs` has a test
    /// for a context with no `authorizer` member. This is what produces one.
    #[test]
    fn an_unauthenticated_route_carries_no_authorizer() {
        let context = unauthenticated(&parts(), "GET /health");
        let view: ServiceView = serde_json::from_str(&context).unwrap();

        assert!(view.authorizer.is_none());
    }
}
