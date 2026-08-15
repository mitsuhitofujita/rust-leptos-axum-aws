//! The one decision the adapter makes about a request, and the conversion it
//! exists to perform.
//!
//! Strip, authorize, attach — or refuse. There is no route table, no preflight
//! and no path of any kind here: the crate reproduces the authorizer's verdict
//! and nothing else about API Gateway, so it has nothing with which to make a
//! second kind of decision (DR-0023).
//!
//! That every request is authorized, `/health` included, is a divergence from
//! the deployment, where the probe is routed outside the authorizer. It is
//! deliberate and is recorded in `workspace.md`, because nothing here can show
//! it: what would show it is an exemption, and there is none.

use axum::body::Body;
use axum::http::StatusCode;
use axum::http::header::{CONTENT_TYPE, HeaderValue};
use axum::http::request::Parts;
use axum::response::Response;
use serde_json::{Map, Value};

use crate::authorizer::{self, Authorization, Verifier};

/// The `AuthContext` the service reads — DR-0024. Both are discarded on the way
/// in, unconditionally, so that neither can be supplied by a caller, and written
/// again only for a request the authorizer accepted.
///
/// In the deployment these are set by API Gateway's request parameter mapping,
/// which `overwrite:`s them for the same reason (DR-0025).
const SUBJECT: &str = "x-auth-subject";
const EDGE: &str = "x-auth-edge";
const AUTH_HEADERS: [&str; 2] = [SUBJECT, EDGE];

/// What the edge header is set to. Nothing reads the value; its presence is the
/// assertion.
const EDGE_VALUE: &str = "devgateway";

pub enum Outcome {
    /// Answered here. The service is never reached.
    Answer(Response),
    /// Forward the request as it now stands.
    Forward,
}

/// What happens to a request, and the changes it carries forward.
///
/// `parts` is modified in place: the auth headers are removed and the
/// `AuthContext` this process produced is put on. That ordering is the point —
/// the removal happens before anything else looks at the request, so there is no
/// path on which a caller's copy survives.
pub fn decide(verifier: &Verifier, parts: &mut Parts) -> Outcome {
    // Production's safety argument is that the edge overwrites these headers on
    // every request (DR-0024, DR-0025). An adapter that passed a caller's copy
    // through would be a rig in which they are forgeable, and would teach the
    // opposite of what is true.
    for header in AUTH_HEADERS {
        parts.headers.remove(header);
    }

    match authorizer::authorize(parts, verifier) {
        Authorization::Refused => Outcome::Answer(unauthorized()),
        Authorization::Allowed(claims) => {
            attach(parts, &claims);
            Outcome::Forward
        }
    }
}

/// The whole of the conversion this crate exists to perform: the claims the
/// authorizer accepted become the `AuthContext` the service reads.
///
/// The edge header goes on unconditionally, including when there is no subject
/// to report. That is the case the service refuses, and reporting it is the
/// point — a token the authorizer accepted but whose subject cannot be named
/// must not be forwarded as though nobody had spoken, because the service would
/// then attribute it to the development owner (DR-0025).
fn attach(parts: &mut Parts, claims: &Map<String, Value>) {
    parts
        .headers
        .insert(EDGE, HeaderValue::from_static(EDGE_VALUE));

    let subject = claims
        .get("sub")
        .and_then(Value::as_str)
        .and_then(|sub| HeaderValue::from_str(sub).ok());

    match subject {
        Some(value) => {
            parts.headers.insert(SUBJECT, value);
        }
        // No `sub`, or one holding a character no header can carry. Either way
        // the edge has asserted an identity it cannot name, which the service
        // answers 401 rather than absorbing.
        None => println!(
            "devgateway: the accepted token carries no usable `sub`; \
             the service will refuse the request"
        ),
    }
}

/// What the deployed authorizer answers, which is what the SPA's `ApiError` sees
/// in production: a fixed body carrying none of the reason — DR-0022.
fn unauthorized() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"message":"Unauthorized"}"#))
        .expect("a valid response")
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::http::Request;

    use super::*;
    use crate::config::Verification;
    use crate::testkey;

    struct Sent {
        parts: Parts,
        outcome: Outcome,
    }

    impl Sent {
        fn status(&self) -> StatusCode {
            match &self.outcome {
                Outcome::Answer(response) => response.status(),
                Outcome::Forward => panic!("the request was forwarded, not answered"),
            }
        }

        fn forwarded(&self) -> bool {
            matches!(self.outcome, Outcome::Forward)
        }

        /// Whether an edge handled the request at all, which is the half of the
        /// `AuthContext` that carries no data.
        fn marked(&self) -> bool {
            assert!(self.forwarded(), "an answered request is never forwarded");
            self.parts.headers.contains_key(EDGE)
        }

        fn subject(&self) -> Option<String> {
            assert!(self.forwarded(), "an answered request is never forwarded");
            self.parts
                .headers
                .get(SUBJECT)
                .map(|value| value.to_str().unwrap().to_owned())
        }
    }

    /// The fixture key set and the fixture's issuer and app client id, which is
    /// what the authorizer's own tests sign against.
    fn verifier() -> Verifier {
        Verifier {
            verification: Verification {
                issuer: testkey::ISSUER.to_owned(),
                audience: testkey::AUDIENCE.to_owned(),
            },
            keys: crate::jwks::Keys::parse(testkey::JWKS.as_bytes()).unwrap(),
        }
    }

    fn send(request: Request<()>) -> Sent {
        let (mut parts, ()) = request.into_parts();
        let outcome = decide(&verifier(), &mut parts);

        Sent { parts, outcome }
    }

    /// A token the fixture key signed, valid against the real clock — which is
    /// the one `authorize` reads. `claims` is the payload minus its braces, so
    /// that a test can leave `sub` out.
    fn token(claims: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        format!(
            "Bearer {}",
            testkey::sign(
                &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
                &format!(
                    r#"{{{claims},"iss":"{}","client_id":"{}","token_use":"access","exp":{}}}"#,
                    testkey::ISSUER,
                    testkey::AUDIENCE,
                    now + 3600,
                ),
            )
        )
    }

    fn subject_token(sub: &str) -> String {
        token(&format!(r#""sub":"{sub}""#))
    }

    /// DR-0010: the request is refused before the function is invoked.
    #[test]
    fn a_request_without_a_token_is_refused() {
        let sent = send(Request::get("/api/action-types").body(()).unwrap());

        assert_eq!(sent.status(), StatusCode::UNAUTHORIZED);
    }

    /// The adapter reproduces the authorizer and not the route table, so a path
    /// the deployment routes outside the authorizer is authorized here like any
    /// other. `/health` through :3001 is a 401; the deployed probe is not.
    #[test]
    fn a_path_outside_api_is_authorized_like_any_other() {
        assert_eq!(
            send(Request::get("/health").body(()).unwrap()).status(),
            StatusCode::UNAUTHORIZED
        );

        let sent = send(
            Request::get("/health")
                .header("authorization", subject_token("abc-123"))
                .body(())
                .unwrap(),
        );
        assert_eq!(sent.subject().as_deref(), Some("abc-123"));
    }

    /// The reason is a developer's, not a caller's. The adapter knows exactly
    /// why it refused and answers what the deployed authorizer answers, which is
    /// a fixed body carrying none of it — DR-0022.
    #[tokio::test]
    async fn a_refusal_says_no_more_than_the_deployed_one() {
        let sent = send(
            Request::get("/api/action-types")
                .header("authorization", "Bearer alice")
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.status(), StatusCode::UNAUTHORIZED);

        let Outcome::Answer(response) = sent.outcome else {
            unreachable!()
        };
        let body = to_bytes(response.into_body(), 1024).await.unwrap();

        assert_eq!(body, r#"{"message":"Unauthorized"}"#);
    }

    /// DR-0024's argument, made visible: the headers are only unforgeable
    /// because the edge overwrites them, so the adapter has to overwrite them
    /// too.
    #[test]
    fn a_forged_context_does_not_survive_a_missing_token() {
        let sent = send(
            Request::get("/api/action-types")
                .header(EDGE, "forged")
                .header(SUBJECT, "attacker")
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn a_forged_context_is_replaced_when_a_token_is_present() {
        let sent = send(
            Request::get("/api/action-types")
                .header(EDGE, "forged")
                .header(SUBJECT, "attacker")
                .header("authorization", subject_token("abc-123"))
                .body(())
                .unwrap(),
        );

        assert_eq!(sent.subject().as_deref(), Some("abc-123"));
    }

    /// A token the authorizer accepted but whose subject cannot be named is the
    /// case DR-0025's second header exists for: the request is marked and
    /// unnamed, which the service refuses rather than attributing to the
    /// development owner.
    #[test]
    fn an_accepted_token_with_no_subject_is_marked_and_unnamed() {
        let sent = send(
            Request::get("/api/action-types")
                .header("authorization", token(r#""scope":"openid""#))
                .body(())
                .unwrap(),
        );

        assert!(sent.marked(), "the edge handled the request");
        assert_eq!(sent.subject(), None, "and could not name the caller");
    }
}
