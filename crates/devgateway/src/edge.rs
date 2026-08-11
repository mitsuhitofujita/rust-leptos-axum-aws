//! The route table, the preflight answer, and the one decision the stand-in
//! makes about a request.
//!
//! Everything here is a copy of `infra/api/apigateway.tf`, kept in step with it
//! by hand — the same arrangement `dynamo_table` and `project` in the `justfile`
//! already have, and for the same reason: nothing local can read Terraform.
//!
//! [`decide`] is a function over the request's parts rather than a router,
//! because an axum router would answer 405 where an HTTP API answers 404, and
//! that difference is one of the things this exists to expose.

use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, HeaderValue};
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::response::Response;

use crate::authorizer::{self, Authorization};
use crate::config::{Config, Mode};
use crate::context;

/// Mirrors `local.api_methods`. A method not in this list has no route under
/// `/api` and is a 404 — not a 405, which is what an axum router would say and
/// what would hide the mismatch.
const API_METHODS: [&str; 2] = ["GET", "POST"];

/// Mirrors `cors_configuration.allow_headers`.
const ALLOW_HEADERS: &str = "authorization,content-type";

/// Mirrors `cors_configuration.max_age`.
const MAX_AGE: &str = "3600";

/// The two headers the Lambda Web Adapter writes. Both are discarded on the way
/// in, unconditionally, so that neither can be supplied by a caller.
const ADAPTER_HEADERS: [&str; 2] = ["x-amzn-request-context", "x-amzn-lambda-context"];

pub enum Outcome {
    /// Answered here. The service is never reached.
    Answer(Response),
    /// Forward the request as it now stands, and put this on the response if it
    /// is there.
    Forward(Option<HeaderValue>),
}

/// What happens to a request, and the changes it carries forward.
///
/// `parts` is modified in place: in `Local` mode the adapter's headers are
/// removed and the request context this process wrote is put on. That ordering
/// is the point — the removal happens before anything else looks at the request,
/// so there is no path on which a caller's copy survives.
pub fn decide(config: &Config, parts: &mut Parts) -> Outcome {
    if config.mode == Mode::Passthrough {
        return Outcome::Forward(None);
    }

    // Production's safety argument is that API Gateway overwrites this header on
    // every request (DR-0017). A rig that passed a caller's copy through would
    // be a mirror in which the header is forgeable, and would teach the opposite
    // of what is true.
    for header in ADAPTER_HEADERS {
        parts.headers.remove(header);
    }

    let allow_origin = allow_origin(config, parts);

    // No OPTIONS route exists, which is exactly why the HTTP API answers
    // preflight itself, ahead of the authorizer — DR-0009. A preflight carries
    // no token and must never be authorized.
    if parts.method == Method::OPTIONS {
        return Outcome::Answer(preflight(allow_origin));
    }

    match route(parts) {
        Route::Unmatched => {
            Outcome::Answer(answer(StatusCode::NOT_FOUND, "Not Found", allow_origin))
        }
        // Routed outside the authorizer, so the context carries no `authorizer`
        // member at all — the shape `identity.rs` already has a test for.
        Route::Health => {
            let context = context::unauthenticated(parts, "GET /health");
            attach(parts, &context);
            Outcome::Forward(allow_origin)
        }
        Route::Api => match authorizer::authorize(parts) {
            Authorization::Refused => Outcome::Answer(answer(
                StatusCode::UNAUTHORIZED,
                "Unauthorized",
                allow_origin,
            )),
            Authorization::Allowed(claims) => {
                let route_key = format!("{} /api/{{proxy+}}", parts.method);
                let context = context::authorized(parts, &route_key, &claims);
                attach(parts, &context);
                Outcome::Forward(allow_origin)
            }
        },
    }
}

enum Route {
    Api,
    Health,
    Unmatched,
}

/// `GET|POST /api/{proxy+}` and `GET /health`, and nothing else.
///
/// `{proxy+}` needs at least one segment after the prefix, so `/api` and `/api/`
/// match nothing either.
fn route(parts: &Parts) -> Route {
    let path = parts.uri.path();
    let method = parts.method.as_str();

    match path.strip_prefix("/api/") {
        Some(rest) if !rest.is_empty() => {
            if API_METHODS.contains(&method) {
                Route::Api
            } else {
                Route::Unmatched
            }
        }
        _ if path == "/health" && method == "GET" => Route::Health,
        _ => Route::Unmatched,
    }
}

fn attach(parts: &mut Parts, context: &str) {
    match HeaderValue::from_str(context) {
        Ok(value) => {
            parts.headers.insert(ADAPTER_HEADERS[0], value);
        }
        // Only reachable if a claim held a control character, which JSON
        // escapes. Refusing to forward is safer than forwarding without it,
        // since without it the service falls back to its development owner.
        Err(error) => println!("devgateway: the request context is not a header value: {error}"),
    }
}

/// An HTTP API echoes the allowed origin on every response, not only on a
/// preflight. Locally the browser talks to trunk, which proxies same-origin, so
/// this is observable with curl and by the tests rather than in a page.
fn allow_origin(config: &Config, parts: &Parts) -> Option<HeaderValue> {
    let origin = parts.headers.get("origin")?;

    (origin.to_str().ok()? == config.allow_origin).then(|| origin.clone())
}

fn preflight(allow_origin: Option<HeaderValue>) -> Response {
    let mut response = Response::builder().status(StatusCode::NO_CONTENT);
    let headers = response
        .headers_mut()
        .expect("the builder has no error yet");

    if let Some(origin) = allow_origin {
        headers.insert("access-control-allow-origin", origin);
        // Mirrors `concat(local.api_methods, ["OPTIONS"])`.
        let methods = format!("{},OPTIONS", API_METHODS.join(","));
        headers.insert(
            "access-control-allow-methods",
            HeaderValue::from_str(&methods).expect("methods are ASCII"),
        );
        headers.insert(
            "access-control-allow-headers",
            HeaderValue::from_static(ALLOW_HEADERS),
        );
        headers.insert("access-control-max-age", HeaderValue::from_static(MAX_AGE));
    }

    response.body(Body::empty()).expect("a valid response")
}

/// The body an HTTP API answers a refusal with, which is what the SPA's
/// `ApiError` sees in production.
fn answer(status: StatusCode, message: &str, allow_origin: Option<HeaderValue>) -> Response {
    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json");

    if let Some(origin) = allow_origin {
        response = response.header("access-control-allow-origin", origin);
    }

    response
        .body(Body::from(format!(r#"{{"message":"{message}"}}"#)))
        .expect("a valid response")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::http::Request;
    use serde_json::Value;

    use super::*;

    struct Sent {
        parts: Parts,
        outcome: Outcome,
    }

    impl Sent {
        fn status(&self) -> StatusCode {
            match &self.outcome {
                Outcome::Answer(response) => response.status(),
                Outcome::Forward(_) => panic!("the request was forwarded, not answered"),
            }
        }

        fn forwarded(&self) -> bool {
            matches!(self.outcome, Outcome::Forward(_))
        }

        fn context(&self) -> Value {
            assert!(self.forwarded(), "an answered request is never forwarded");
            let context = self.parts.headers[ADAPTER_HEADERS[0]].to_str().unwrap();
            serde_json::from_str(context).unwrap()
        }

        fn subject(&self) -> Option<String> {
            let context = self.context();
            let claims: HashMap<String, String> =
                serde_json::from_value(context["authorizer"]["jwt"]["claims"].clone()).unwrap();
            claims.get("sub").cloned()
        }
    }

    fn send(mode: Mode, request: Request<()>) -> Sent {
        let config = Config::for_test(mode);
        let (mut parts, ()) = request.into_parts();
        let outcome = decide(&config, &mut parts);

        Sent { parts, outcome }
    }

    fn get(path: &str) -> Request<()> {
        Request::get(path).body(()).unwrap()
    }

    /// A JWT this stand-in built. Nothing verifies it, here or in the mode under
    /// test.
    fn token(sub: &str) -> String {
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;

        format!(
            "Bearer {}.{}.{}",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#),
            URL_SAFE_NO_PAD.encode(format!(r#"{{"sub":"{sub}"}}"#)),
            URL_SAFE_NO_PAD.encode("signature"),
        )
    }

    /// The distinction a stand-in built on a router would lose: an HTTP API has
    /// no route for a method outside `local.api_methods`, so it answers 404
    /// where a router would answer 405, and the service is never reached.
    #[test]
    fn a_method_outside_the_route_table_is_not_found_rather_than_not_allowed() {
        let sent = send(
            Mode::Local,
            Request::delete("/api/action-types")
                .header("authorization", token("abc-123"))
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_path_outside_the_route_table_is_not_found() {
        for path in ["/", "/nope", "/api", "/api/"] {
            let sent = send(Mode::Local, get(path));
            assert_eq!(sent.status(), StatusCode::NOT_FOUND, "for {path}");
        }
    }

    /// DR-0010: the request is refused before the function is invoked.
    #[test]
    fn a_request_under_api_without_a_token_is_refused() {
        let sent = send(Mode::Local, get("/api/action-types"));

        assert_eq!(sent.status(), StatusCode::UNAUTHORIZED);
    }

    /// The probe carries no token by design and is routed outside the
    /// authorizer.
    #[test]
    fn the_probe_is_forwarded_without_a_token() {
        let sent = send(Mode::Local, get("/health"));

        assert!(sent.forwarded());
        assert!(sent.context().get("authorizer").is_none());
    }

    /// DR-0017's argument, made visible: the header is only unforgeable because
    /// the edge overwrites it, so the stand-in has to overwrite it too.
    #[test]
    fn a_forged_request_context_does_not_survive_a_missing_token() {
        let sent = send(
            Mode::Local,
            Request::get("/api/action-types")
                .header(
                    "x-amzn-request-context",
                    r#"{"authorizer":{"jwt":{"claims":{"sub":"attacker"}}}}"#,
                )
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn a_forged_request_context_is_replaced_when_a_token_is_present() {
        let sent = send(
            Mode::Local,
            Request::get("/api/action-types")
                .header(
                    "x-amzn-request-context",
                    r#"{"authorizer":{"jwt":{"claims":{"sub":"attacker"}}}}"#,
                )
                .header("authorization", token("abc-123"))
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.subject().as_deref(), Some("abc-123"));
    }

    #[test]
    fn the_adapters_lambda_context_is_discarded_too() {
        let sent = send(
            Mode::Local,
            Request::get("/health")
                .header("x-amzn-lambda-context", r#"{"request_id":"forged"}"#)
                .body(())
                .unwrap(),
        );

        assert!(!sent.parts.headers.contains_key("x-amzn-lambda-context"));
    }

    /// Two callers, one `curl` flag apart. Nothing before this could observe
    /// that one owner does not see another's items without deploying.
    #[test]
    fn a_bearer_value_that_is_not_a_jwt_is_the_subject_itself() {
        for caller in ["alice", "bob"] {
            let sent = send(
                Mode::Local,
                Request::get("/api/action-types")
                    .header("authorization", format!("Bearer {caller}"))
                    .body(())
                    .unwrap(),
            );

            assert_eq!(sent.subject().as_deref(), Some(caller));
        }
    }

    /// DR-0009: preflight is answered ahead of the authorizer, so it needs no
    /// token. An `ANY` route would put the authorizer in front of it and answer
    /// 401, blocking the request it precedes.
    #[test]
    fn a_preflight_is_answered_here_and_never_authorized() {
        let sent = send(
            Mode::Local,
            Request::options("/api/action-types")
                .header("origin", "http://localhost:8080")
                .header("access-control-request-method", "POST")
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.status(), StatusCode::NO_CONTENT);

        let Outcome::Answer(response) = &sent.outcome else {
            unreachable!()
        };
        let headers = response.headers();
        assert_eq!(
            headers["access-control-allow-origin"],
            "http://localhost:8080"
        );
        assert_eq!(headers["access-control-allow-methods"], "GET,POST,OPTIONS");
        assert_eq!(headers["access-control-allow-headers"], ALLOW_HEADERS);
    }

    #[test]
    fn a_preflight_from_an_unlisted_origin_is_answered_without_the_allow_header() {
        let sent = send(
            Mode::Local,
            Request::options("/api/action-types")
                .header("origin", "http://evil.example")
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.status(), StatusCode::NO_CONTENT);

        let Outcome::Answer(response) = &sent.outcome else {
            unreachable!()
        };
        assert!(
            !response
                .headers()
                .contains_key("access-control-allow-origin")
        );
    }

    /// `passthrough` is the absence of a mirror rather than a second one: it is
    /// what `just dev-api` on its own already does, forged header and all.
    #[test]
    fn passthrough_adds_nothing_and_removes_nothing() {
        let sent = send(
            Mode::Passthrough,
            Request::delete("/api/action-types")
                .header("x-amzn-request-context", r#"{"accountId":"1"}"#)
                .body(())
                .unwrap(),
        );

        assert!(sent.forwarded());
        assert_eq!(
            sent.parts.headers["x-amzn-request-context"],
            r#"{"accountId":"1"}"#
        );
    }
}
