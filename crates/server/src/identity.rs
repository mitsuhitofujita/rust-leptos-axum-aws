//! Who the request is from.
//!
//! Two arrangements, chosen once at startup by [`Auth::from_environment`],
//! the same shape [`crate::store::Store`] already uses for the data layer:
//! `Auth::Cognito` verifies a real token against the pool, `Auth::Mock`
//! trusts two headers set by hand. Whichever is active, a handler asks for
//! [`Owner`] and gets the same [`AuthContext`] shape back — DR-0024's claim
//! that nothing downstream of verification needs to change still holds.
//!
//! **The safety argument is which variant is running, not anything upstream.**
//! Before DR-0028, the two headers this file reads were safe because API
//! Gateway's request-parameter mapping overwrote them on every request; that
//! mapping is gone. What replaces it: `Auth::Mock` is the only variant that
//! ever reads those headers, and it is only the active variant when
//! `COGNITO_ISSUER`/`COGNITO_AUDIENCE` are both unset — which the deployed
//! Lambda never has, because Terraform always sets both, mirroring
//! `TABLE_NAME`. So the deployed function is always `Auth::Cognito`, and
//! `Auth::Cognito` never looks at either header at all, whatever a caller
//! sends.

use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::cognito;

/// The caller's subject, as `Auth::Mock` asserts it by hand.
const SUBJECT: &str = "x-auth-subject";

/// Set by a caller under `Auth::Mock`, to a value nobody reads.
///
/// Its presence is the whole of its meaning: it says a caller means to name
/// someone. Without it, a [`SUBJECT`] that failed to arrive would be
/// indistinguishable from no assertion at all, and a request that named
/// nobody would be attributed to [`DEVELOPMENT_OWNER`] rather than refused.
const EDGE: &str = "x-auth-edge";

/// Who owns the items a request reads and writes when `Auth::Mock` is active
/// and nothing says otherwise.
///
/// There is no edge under `just dev-api`: no API Gateway, no token, and
/// `COGNITO_ISSUER`/`COGNITO_AUDIENCE` are unset, so `Auth::Mock` is what
/// runs. An unset value meaning something workable rather than something
/// broken is the same choice DR-0008 makes for the frontend's configuration,
/// and this is its equivalent here (DR-0018). This rule is scoped to
/// `Auth::Mock` alone: under `Auth::Cognito`, a missing token is refused,
/// with no fallback — see [`Auth::Cognito`].
///
/// It is a partition key like any other, so development data sits beside
/// real data without mixing with it.
const DEVELOPMENT_OWNER: &str = "development";

/// What the service knows about whoever made the request.
///
/// One field, because one field is what a handler uses. DR-0024 describes
/// this as "a subject, and whatever else a handler genuinely uses", and
/// nothing yet uses anything else.
pub struct AuthContext {
    /// The Cognito subject. A `sub` rather than an email address: it is what
    /// the token asserts, and unlike an address it does not change (DR-0015).
    pub subject: String,
}

/// The owner of everything a request touches.
pub struct Owner(pub String);

/// Chosen once at startup, exactly like [`crate::store::Store`]: which
/// arrangement is running does not change while the process runs.
pub enum Auth {
    /// A real Cognito token, verified against the pool the way
    /// `aws_apigatewayv2_authorizer.cognito` verified it before DR-0028
    /// retired it.
    Cognito(cognito::Verifier),
    /// The two headers below, set by hand. What `just dev-api` runs with no
    /// configuration at all.
    Mock,
}

impl Auth {
    /// `COGNITO_ISSUER` and `COGNITO_AUDIENCE` unset means `Mock`; both set
    /// means `Cognito`, and the pool's key set is fetched here, before the
    /// listener binds — a pool that cannot be reached is a reason to stop,
    /// with the reason on screen, not to accept connections and refuse every
    /// one of them for a cause nobody can see.
    ///
    /// Exactly one set is refused outright rather than treated as unset:
    /// falling back to `Mock` on a half-configured environment would
    /// downgrade a deployed function to header-trusting mode on a typo,
    /// which is the exact failure this arrangement exists to rule out
    /// structurally.
    pub async fn from_environment() -> Result<Self, String> {
        match (env_var("COGNITO_ISSUER"), env_var("COGNITO_AUDIENCE")) {
            (None, None) => Ok(Self::Mock),
            (Some(issuer), Some(audience)) => {
                let keys = crate::jwks::Keys::fetch(&issuer).await?;
                Ok(Self::Cognito(cognito::Verifier {
                    verification: cognito::Verification { issuer, audience },
                    keys,
                }))
            }
            (Some(_), None) => {
                Err("COGNITO_ISSUER is set but COGNITO_AUDIENCE is not; both or neither".to_owned())
            }
            (None, Some(_)) => {
                Err("COGNITO_AUDIENCE is set but COGNITO_ISSUER is not; both or neither".to_owned())
            }
        }
    }

    /// The single most useful thing to know about a running instance,
    /// mirroring [`crate::store::Store::describe`].
    pub fn describe(&self) -> String {
        match self {
            Self::Cognito(verifier) => format!(
                "Cognito, {} key(s) from {}",
                verifier.keys.len(),
                verifier.verification.issuer
            ),
            Self::Mock => {
                format!("the {SUBJECT}/{EDGE} headers (COGNITO_ISSUER is unset)")
            }
        }
    }
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

impl<S> FromRequestParts<S> for Owner
where
    Arc<Auth>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = NoSubject;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match &*Arc::<Auth>::from_ref(state) {
            Auth::Mock => match mock_caller(&parts.headers) {
                Caller::Known(context) => Ok(Self(context.subject)),
                Caller::Developer => Ok(Self(DEVELOPMENT_OWNER.to_owned())),
                Caller::Unreadable => Err(NoSubject),
            },
            Auth::Cognito(verifier) => verified_subject(parts, verifier).map(Self).ok_or(NoSubject),
        }
    }
}

/// An edge asserted an identity and the service could not read it, or a
/// caller under `Auth::Cognito` sent no token, or a bad one.
///
/// Answered `401` rather than absorbed. Treating any of these as "nobody
/// said anything" is how a write reaches the wrong partition silently, and
/// that is the failure DR-0024 and DR-0028 exist to remove.
pub struct NoSubject;

impl IntoResponse for NoSubject {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}

enum Caller {
    Known(AuthContext),
    /// Nothing asserted anything, so there is nothing to misread.
    Developer,
    /// Something did assert, and what arrived is not a subject.
    Unreadable,
}

/// The three cases `Auth::Mock` distinguishes, and the difference between the
/// last two is the point.
fn mock_caller(headers: &HeaderMap) -> Caller {
    if !headers.contains_key(EDGE) {
        return Caller::Developer;
    }

    // An empty value is what a caller sends when it has an identity to
    // assert and cannot name it, so it is unreadable rather than absent.
    match headers.get(SUBJECT).and_then(|value| value.to_str().ok()) {
        Some(subject) if !subject.is_empty() => Caller::Known(AuthContext {
            subject: subject.to_owned(),
        }),
        _ => Caller::Unreadable,
    }
}

/// The identity source is `Authorization`, exactly as
/// `$request.header.Authorization` was. The `Bearer` prefix is optional
/// there and so it is here.
fn bearer(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get("authorization")?.to_str().ok()?;
    let token = match value.split_once(' ') {
        Some((scheme, token)) if scheme.eq_ignore_ascii_case("bearer") => token.trim(),
        _ => value.trim(),
    };

    (!token.is_empty()).then_some(token)
}

/// `Auth::Cognito`'s whole rule: no token, or a token that does not verify,
/// or one with no usable `sub`, is a refusal — never the development owner.
fn verified_subject(parts: &Parts, verifier: &cognito::Verifier) -> Option<String> {
    let token = bearer(parts)?;

    match verifier.verified(token, cognito::unix_time()) {
        Ok(claims) => {
            verifier.report(&claims);
            claims
                .get("sub")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        }
        Err(reason) => {
            println!("server: 401 — {reason}");
            None
        }
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
        match mock_caller(&headers(pairs)) {
            Caller::Known(context) => Some(context.subject),
            Caller::Developer => Some(DEVELOPMENT_OWNER.to_owned()),
            Caller::Unreadable => None,
        }
    }

    #[test]
    fn reads_the_subject_a_mock_caller_asserted() {
        assert_eq!(
            owner(&[(EDGE, "apigateway"), (SUBJECT, "abc-123")]).as_deref(),
            Some("abc-123")
        );
    }

    /// DR-0018, unchanged: under `Auth::Mock`, a request nobody spoke for is
    /// a developer's own.
    #[test]
    fn a_request_no_edge_touched_is_the_development_owner() {
        assert_eq!(owner(&[]).as_deref(), Some(DEVELOPMENT_OWNER));
    }

    /// The case this whole boundary exists to make expressible. A caller
    /// said it had an identity and did not name them, which must not become
    /// a write into the development owner's partition.
    #[test]
    fn an_edge_that_named_nobody_is_refused() {
        assert_eq!(owner(&[(EDGE, "apigateway")]), None);
    }

    /// The same failure with the header present but empty, which is what a
    /// mapping that resolved to nothing used to produce.
    #[test]
    fn an_edge_that_named_an_empty_subject_is_refused() {
        assert_eq!(owner(&[(EDGE, "apigateway"), (SUBJECT, "")]), None);
    }

    /// An unmarked request is a developer's however it is dressed: the
    /// subject header alone asserts nothing, because nothing claimed to have
    /// handled the request. Being `alice` under `just dev-api` therefore
    /// means sending both headers, which is the whole of what `Auth::Mock`
    /// is.
    #[test]
    fn a_subject_without_an_edge_is_still_the_development_owner() {
        assert_eq!(
            owner(&[(SUBJECT, "alice")]).as_deref(),
            Some(DEVELOPMENT_OWNER)
        );
    }

    // --- Auth::Cognito ------------------------------------------------------
    //
    // The audience/issuer/expiry rule itself is `cognito.rs`'s to test; these
    // check only what is specific to this file: bearer extraction and the
    // no-fallback rule.

    use crate::jwks::Keys;
    use crate::testkey;

    fn verifier() -> cognito::Verifier {
        cognito::Verifier {
            verification: cognito::Verification {
                issuer: testkey::ISSUER.to_owned(),
                audience: testkey::AUDIENCE.to_owned(),
            },
            keys: Keys::parse(testkey::JWKS.as_bytes()).unwrap(),
        }
    }

    fn parts(authorization: Option<&str>) -> Parts {
        let mut request = axum::http::Request::get("/api/action-types");
        if let Some(value) = authorization {
            request = request.header("authorization", value);
        }
        request.body(()).unwrap().into_parts().0
    }

    /// A well-formed access token whose `exp` is against the real clock,
    /// which is the one `verified_subject` reads.
    fn current_token() -> String {
        testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","client_id":"{}","token_use":"access","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                cognito::unix_time() + 3600,
            ),
        )
    }

    /// No fallback: a missing token under `Auth::Cognito` is a refusal, never
    /// the development owner. This is the single highest-value case in this
    /// file — getting it backwards means every unauthenticated caller against
    /// a deployed function reaches a working, shared partition.
    #[test]
    fn no_token_under_cognito_is_refused_not_the_development_owner() {
        assert_eq!(verified_subject(&parts(None), &verifier()), None);
    }

    #[test]
    fn a_bare_name_is_refused_under_cognito() {
        assert_eq!(
            verified_subject(&parts(Some("Bearer alice")), &verifier()),
            None
        );
    }

    #[test]
    fn a_verified_token_yields_its_subject() {
        assert_eq!(
            verified_subject(
                &parts(Some(&format!("Bearer {}", current_token()))),
                &verifier()
            )
            .as_deref(),
            Some("abc-123")
        );
    }

    /// The prefix is optional, matching what `$request.header.Authorization`
    /// accepted.
    #[test]
    fn a_token_without_the_bearer_prefix_is_accepted() {
        assert_eq!(
            verified_subject(&parts(Some(&current_token())), &verifier()).as_deref(),
            Some("abc-123")
        );
    }
}
