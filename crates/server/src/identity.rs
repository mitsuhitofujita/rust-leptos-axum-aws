//! Who the request is from.
//!
//! The service never validates a token. API Gateway's JWT authorizer has
//! already done it, and it is the only enforcement point there is (DR-0010).
//! What reaches the function is the authorizer's conclusion — but not in AWS's
//! shape. Whatever sits in front converts it into an [`AuthContext`] first, and
//! that is the only description of a caller this crate has (DR-0024).
//!
//! Nothing here names API Gateway, a request context, a JWT or a claim. In the
//! deployment the conversion is API Gateway's own request parameter mapping, and
//! locally it is `crates/devgateway`; both produce the same two headers, which
//! is what makes them interchangeable rather than two code paths (DR-0025).
//!
//! Nothing here is a security boundary. A header can be forged by anyone who
//! can reach the service directly; what makes that irrelevant is that nothing
//! can — API Gateway is the only route to the function, and its mapping
//! overwrites both headers on every request.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

/// The caller's subject, as whatever is in front of the service asserts it.
const SUBJECT: &str = "x-auth-subject";

/// Set by whatever is in front of the service, to a value nobody reads.
///
/// Its presence is the whole of its meaning: it says an edge handled this
/// request. Without it, a [`SUBJECT`] that failed to arrive would be
/// indistinguishable from no edge at all, and a request the edge could not name
/// would be attributed to [`DEVELOPMENT_OWNER`] rather than refused — DR-0025.
const EDGE: &str = "x-auth-edge";

/// Who owns the items a request reads and writes when nothing says otherwise.
///
/// There is no edge under `just dev-api`: no API Gateway, no authorizer, and no
/// token — the SPA is not even configured to obtain one unless
/// `just dev-web-auth` is what started it. An unset value meaning something
/// workable rather than something broken is the same choice DR-0008 makes for
/// the frontend's configuration, and this is its equivalent here (DR-0018).
///
/// It is a partition key like any other, so development data sits beside real
/// data without mixing with it. In a deployed function this is unreachable:
/// every request arrives through the edge, which is what sets [`EDGE`].
const DEVELOPMENT_OWNER: &str = "development";

/// What the service knows about whoever made the request.
///
/// One field, because one field is what a handler uses. DR-0024 describes this
/// as "a subject, and whatever else a handler genuinely uses", and nothing yet
/// uses anything else.
pub struct AuthContext {
    /// The Cognito subject. A `sub` rather than an email address: it is what the
    /// token asserts, and unlike an address it does not change (DR-0015).
    pub subject: String,
}

/// The owner of everything a request touches.
pub struct Owner(pub String);

impl<S: Send + Sync> FromRequestParts<S> for Owner {
    type Rejection = NoSubject;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match context(&parts.headers) {
            Caller::Known(context) => Ok(Self(context.subject)),
            Caller::Developer => Ok(Self(DEVELOPMENT_OWNER.to_owned())),
            Caller::Unreadable => Err(NoSubject),
        }
    }
}

/// An edge asserted an identity and the service could not read it.
///
/// Answered `401` rather than absorbed. Treating it as "nobody said anything"
/// is how a write reaches the wrong partition silently, and that is the failure
/// DR-0024 exists to remove.
pub struct NoSubject;

impl IntoResponse for NoSubject {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}

enum Caller {
    Known(AuthContext),
    /// Nothing in front asserted anything, so there is nothing to misread.
    Developer,
    /// Something in front did assert, and what arrived is not a subject.
    Unreadable,
}

/// The three cases, and the difference between the last two is the point.
fn context(headers: &HeaderMap) -> Caller {
    if !headers.contains_key(EDGE) {
        return Caller::Developer;
    }

    // An empty value is what an edge produces when it has an identity to assert
    // and cannot name it, so it is unreadable rather than absent.
    match headers.get(SUBJECT).and_then(|value| value.to_str().ok()) {
        Some(subject) if !subject.is_empty() => Caller::Known(AuthContext {
            subject: subject.to_owned(),
        }),
        _ => Caller::Unreadable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(*name, value.parse().unwrap());
        }
        headers
    }

    fn owner(pairs: &[(&'static str, &str)]) -> Option<String> {
        match context(&headers(pairs)) {
            Caller::Known(context) => Some(context.subject),
            Caller::Developer => Some(DEVELOPMENT_OWNER.to_owned()),
            Caller::Unreadable => None,
        }
    }

    #[test]
    fn reads_the_subject_the_edge_asserted() {
        assert_eq!(
            owner(&[(EDGE, "apigateway"), (SUBJECT, "abc-123")]).as_deref(),
            Some("abc-123")
        );
    }

    /// DR-0018, unchanged: a request nothing spoke for is a developer's own.
    #[test]
    fn a_request_no_edge_touched_is_the_development_owner() {
        assert_eq!(owner(&[]).as_deref(), Some(DEVELOPMENT_OWNER));
    }

    /// The case this whole boundary exists to make expressible. An edge said it
    /// had a caller and did not name them, which must not become a write into
    /// the development owner's partition.
    #[test]
    fn an_edge_that_named_nobody_is_refused() {
        assert_eq!(owner(&[(EDGE, "apigateway")]), None);
    }

    /// The same failure with the header present but empty, which is what a
    /// mapping that resolved to nothing produces.
    #[test]
    fn an_edge_that_named_an_empty_subject_is_refused() {
        assert_eq!(owner(&[(EDGE, "apigateway"), (SUBJECT, "")]), None);
    }

    /// An unmarked request is a developer's however it is dressed: the subject
    /// header alone asserts nothing, because nothing claimed to have handled the
    /// request. Being `alice` under `just dev-api` therefore means sending both
    /// headers, which is the whole of what mock authentication is.
    #[test]
    fn a_subject_without_an_edge_is_still_the_development_owner() {
        assert_eq!(
            owner(&[(SUBJECT, "alice")]).as_deref(),
            Some(DEVELOPMENT_OWNER)
        );
    }
}
