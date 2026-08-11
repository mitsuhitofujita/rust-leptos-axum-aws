//! The half of the stand-in that plays `aws_apigatewayv2_authorizer.cognito`.
//!
//! It decodes; it does not verify. That is enough to exercise everything
//! downstream of the authorizer — the request context, the subject, and the
//! isolation `identity::Owner` provides — and says nothing about whether the
//! deployed authorizer would have accepted the same token. Verifying against the
//! pool's JWKS is a later phase.
//!
//! Nothing here is a security boundary, and it is not trying to be one. It
//! exists so that the *shape* of what the service receives is the deployed
//! shape, and so that a request with no token is refused where the deployed one
//! refuses it.

use axum::http::request::Parts;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Map, Value};

pub enum Authorization {
    /// The claims the authorizer concluded with, before they are stringified.
    Allowed(Map<String, Value>),
    /// No token. The deployed authorizer answers 401 and the function is never
    /// invoked; so does this.
    Refused,
}

pub fn authorize(parts: &Parts) -> Authorization {
    let Some(token) = bearer(parts) else {
        return Authorization::Refused;
    };

    match claims(token) {
        Some(claims) => {
            if !claims.contains_key("sub") {
                println!(
                    "devgateway: the token carries no `sub`; \
                     the service will fall back to its development owner"
                );
            }
            Authorization::Allowed(claims)
        }
        // Not a JWT, so the bearer value is taken as the subject itself:
        // `Bearer alice` is a caller named alice. That is what makes two callers
        // expressible without two sign-ins, and checking that one cannot see the
        // other's items is the whole reason `identity::Owner` exists.
        //
        // A real token that failed to decode lands here too, and its subject is
        // then the whole token — visible in the partition rather than silent.
        None => {
            let mut claims = Map::new();
            claims.insert("sub".to_owned(), Value::String(token.to_owned()));
            Authorization::Allowed(claims)
        }
    }
}

/// The identity source is `$request.header.Authorization` — `apigateway.tf`.
/// The `Bearer` prefix is optional there and so it is here.
fn bearer(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get("authorization")?.to_str().ok()?;
    let token = match value.split_once(' ') {
        Some((scheme, token)) if scheme.eq_ignore_ascii_case("bearer") => token.trim(),
        _ => value.trim(),
    };

    (!token.is_empty()).then_some(token)
}

/// The payload of a JWT, read without verifying anything about it.
fn claims(token: &str) -> Option<Map<String, Value>> {
    let mut segments = token.split('.');
    let (_header, payload, _signature) = (segments.next()?, segments.next()?, segments.next()?);

    if segments.next().is_some() {
        return None;
    }

    let payload = URL_SAFE_NO_PAD.decode(payload).ok()?;
    match serde_json::from_slice(&payload).ok()? {
        Value::Object(claims) => Some(claims),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use axum::http::Request;

    use super::*;

    fn parts(authorization: Option<&str>) -> Parts {
        let mut request = Request::get("/api/action-types");
        if let Some(value) = authorization {
            request = request.header("authorization", value);
        }
        request.body(()).unwrap().into_parts().0
    }

    fn allowed(authorization: Option<&str>) -> Map<String, Value> {
        match authorize(&parts(authorization)) {
            Authorization::Allowed(claims) => claims,
            Authorization::Refused => panic!("expected the request to be authorized"),
        }
    }

    /// A token this stand-in built, so the test does not depend on a real one.
    fn token(payload: &str) -> String {
        format!(
            "{}.{}.{}",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","kid":"local"}"#),
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode("not a signature"),
        )
    }

    #[test]
    fn refuses_a_request_with_no_authorization_header() {
        assert!(matches!(authorize(&parts(None)), Authorization::Refused));
    }

    #[test]
    fn refuses_an_empty_authorization_header() {
        assert!(matches!(
            authorize(&parts(Some("Bearer "))),
            Authorization::Refused
        ));
    }

    #[test]
    fn reads_the_claims_of_a_jwt_without_verifying_it() {
        let claims = allowed(Some(&format!(
            "Bearer {}",
            token(r#"{"sub":"abc-123","client_id":"x"}"#)
        )));

        assert_eq!(
            claims.get("sub"),
            Some(&Value::String("abc-123".to_owned()))
        );
    }

    /// The signature is never looked at, which is the whole of what this mode
    /// does not do. A later phase adds the mode that would reject this.
    #[test]
    fn accepts_a_jwt_whose_signature_is_nonsense() {
        let mut token = token(r#"{"sub":"abc-123"}"#);
        token.push_str("tampered");

        let claims = allowed(Some(&format!("Bearer {token}")));

        assert_eq!(
            claims.get("sub"),
            Some(&Value::String("abc-123".to_owned()))
        );
    }

    /// Two callers without two sign-ins, which is what makes the isolation
    /// `identity::Owner` provides checkable by hand for the first time.
    #[test]
    fn a_bearer_value_that_is_not_a_jwt_is_the_subject_itself() {
        assert_eq!(
            allowed(Some("Bearer alice")).get("sub"),
            Some(&Value::String("alice".to_owned()))
        );
        assert_eq!(
            allowed(Some("Bearer bob")).get("sub"),
            Some(&Value::String("bob".to_owned()))
        );
    }

    /// The prefix is optional at the deployed authorizer's identity source, so a
    /// bare token is a caller here too.
    #[test]
    fn accepts_a_token_without_the_bearer_prefix() {
        let claims = allowed(Some(&token(r#"{"sub":"abc-123"}"#)));

        assert_eq!(
            claims.get("sub"),
            Some(&Value::String("abc-123".to_owned()))
        );
    }
}
