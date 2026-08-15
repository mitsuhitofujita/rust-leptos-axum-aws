//! The half of the adapter that plays `aws_apigatewayv2_authorizer.cognito`.
//!
//! It does what the deployed authorizer does: RS256 against the pool's published
//! key, then `iss`, `exp` and the audience — DR-0022. Nothing here is a security
//! boundary. It exists so that a wrong `jwt_configuration` in
//! `infra/api/apigateway.tf` is visible before an apply instead of arriving
//! afterwards as a 401 indistinguishable from a broken sign-in.
//!
//! There is no mode in which a token is decoded without being verified. That
//! existed to make `Bearer alice` and `Bearer bob` two callers, and DR-0024
//! moved that affordance to `just dev-api`, where the two `AuthContext` headers
//! say the same thing without a token, an adapter or the network.
//!
//! Every refusal is the same refusal: the caller is told `401
//! {"message":"Unauthorized"}` and nothing else, exactly as the deployed
//! authorizer tells them, and the reason is printed on this process's terminal.
//! The reason is what a developer needs and what a caller must not have.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::request::Parts;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Map, Value};

use crate::config::Verification;
use crate::jwks::Keys;

pub enum Authorization {
    /// The claims the authorizer concluded with, which the `AuthContext` is
    /// built from.
    Allowed(Map<String, Value>),
    /// No token, or one that did not verify. The deployed authorizer answers 401
    /// and the function is never invoked; so does this.
    Refused,
}

/// What the adapter verifies against: the deployed authorizer's configuration,
/// and the keys the pool publishes.
pub struct Verifier {
    pub verification: Verification,
    pub keys: Keys,
}

pub fn authorize(parts: &Parts, verifier: &Verifier) -> Authorization {
    let Some(token) = bearer(parts) else {
        return Authorization::Refused;
    };

    match verifier.verified(token, unix_time()) {
        Ok(claims) => {
            verifier.report(&claims);
            Authorization::Allowed(claims)
        }
        Err(reason) => {
            println!("devgateway: 401 — {reason}");
            Authorization::Refused
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

impl Verifier {
    /// The deployed authorizer's checks, in the order it is worth doing them.
    ///
    /// The signature comes before any claim is read, so nothing downstream ever
    /// acts on a payload that has not been established as the pool's.
    fn verified(&self, token: &str, now: u64) -> Result<Map<String, Value>, String> {
        let mut segments = token.split('.');
        let (Some(header), Some(payload), Some(signature), None) = (
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
        ) else {
            return Err(
                "the bearer value is not a JWT. There is nothing to verify a bare \
                 name against; `Bearer alice` is `just dev-api` and its two \
                 AuthContext headers, not this adapter"
                    .to_owned(),
            );
        };

        let head = decode_json(header).ok_or("the token's header is not base64url JSON")?;

        // The deployed authorizer validates RS256 and nothing else. `none` in
        // particular has to be refused here rather than fall through to a
        // missing-key error, because a token asserting it is the one attack this
        // shape has.
        match head.get("alg").and_then(Value::as_str) {
            Some("RS256") => {}
            other => {
                return Err(format!(
                    "the token's `alg` is {}; a Cognito token is signed RS256",
                    describe(other)
                ));
            }
        }

        let kid = head
            .get("kid")
            .and_then(Value::as_str)
            .ok_or("the token's header carries no `kid`, so no key can be selected")?;

        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| "the token's signature is not base64url".to_owned())?;

        let signing_input = format!("{header}.{payload}");
        self.keys
            .verify(kid, signing_input.as_bytes(), &signature)?;

        let claims = decode_json(payload).ok_or("the token's payload is not base64url JSON")?;

        self.check_issuer(&claims)?;
        self.check_validity(&claims, now)?;
        self.check_audience(&claims)?;

        Ok(claims)
    }

    /// `jwt_configuration.issuer`, matched exactly — which is what API Gateway
    /// does, and the reason a trailing slash or a missing `https://` in the
    /// Terraform value refuses every token.
    fn check_issuer(&self, claims: &Map<String, Value>) -> Result<(), String> {
        let iss = claims.get("iss").and_then(Value::as_str);

        if iss == Some(self.verification.issuer.as_str()) {
            return Ok(());
        }

        Err(format!(
            "the token's `iss` is {}; the configured issuer is `{}`",
            describe(iss),
            self.verification.issuer
        ))
    }

    /// `exp`, and `nbf` when the token carries one. Cognito issues neither as a
    /// string, so a value that is not a number is a token from somewhere else.
    fn check_validity(&self, claims: &Map<String, Value>, now: u64) -> Result<(), String> {
        let exp = claims
            .get("exp")
            .and_then(Value::as_u64)
            .ok_or("the token carries no numeric `exp`")?;

        if exp <= now {
            return Err(format!(
                "the token expired {} seconds ago. A Cognito access token lasts \
                 an hour; sign in again",
                now - exp
            ));
        }

        match claims.get("nbf").and_then(Value::as_u64) {
            Some(nbf) if nbf > now => Err(format!(
                "the token is not valid for another {} seconds",
                nbf - now
            )),
            _ => Ok(()),
        }
    }

    /// **The check this whole phase exists for.**
    ///
    /// `jwt_configuration.audience` holds the app client id, and the claim it
    /// arrives in depends on which token the SPA sent: a Cognito **access**
    /// token carries it as `client_id`, an **id** token as `aud`. API Gateway
    /// accepts either, so an adapter that looked only at `aud` would refuse what
    /// the deployment accepts, and one that looked only at `client_id` would
    /// accept what it refuses. Both mistakes are invisible until an apply.
    ///
    /// `aud` is a string or an array of them, per RFC 7519.
    fn check_audience(&self, claims: &Map<String, Value>) -> Result<(), String> {
        let wanted = self.verification.audience.as_str();

        let matched = match claims.get("aud") {
            Some(Value::String(aud)) => aud == wanted,
            Some(Value::Array(auds)) => auds.iter().any(|aud| aud.as_str() == Some(wanted)),
            _ => false,
        } || claims.get("client_id").and_then(Value::as_str) == Some(wanted);

        if matched {
            return Ok(());
        }

        Err(format!(
            "neither the token's `aud` ({}) nor its `client_id` ({}) is the \
             configured app client `{wanted}`. This is a {} token",
            describe_value(claims.get("aud")),
            describe(claims.get("client_id").and_then(Value::as_str)),
            token_use(claims),
        ))
    }

    /// One line per accepted token, naming which kind arrived and which claim
    /// satisfied the audience.
    ///
    /// This is not logging for its own sake. "The SPA is sending the id token"
    /// and "the SPA is sending the access token" are the two states this phase
    /// was built to tell apart, and the difference is invisible in a 200.
    fn report(&self, claims: &Map<String, Value>) {
        let wanted = self.verification.audience.as_str();
        let matched = if claims.get("client_id").and_then(Value::as_str) == Some(wanted) {
            "client_id"
        } else {
            "aud"
        };

        println!(
            "devgateway: 200 — verified {} token for sub={}, audience matched on `{matched}`",
            token_use(claims),
            claims.get("sub").and_then(Value::as_str).unwrap_or("?"),
        );
    }
}

/// What Cognito calls the kind of token, when it says so.
fn token_use(claims: &Map<String, Value>) -> &str {
    claims
        .get("token_use")
        .and_then(Value::as_str)
        .unwrap_or("token_use-less")
}

fn decode_json(segment: &str) -> Option<Map<String, Value>> {
    let bytes = URL_SAFE_NO_PAD.decode(segment).ok()?;
    match serde_json::from_slice(&bytes).ok()? {
        Value::Object(fields) => Some(fields),
        _ => None,
    }
}

/// An absent value reads as `absent` rather than as an empty pair of backticks,
/// because "the `iss` is `` " is the one message that would send a reader
/// looking for a bug in this file.
fn describe(value: Option<&str>) -> String {
    value.map_or_else(|| "absent".to_owned(), |value| format!("`{value}`"))
}

fn describe_value(value: Option<&Value>) -> String {
    value.map_or_else(|| "absent".to_owned(), |value| format!("`{value}`"))
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use axum::http::Request;

    use super::*;
    use crate::testkey;

    fn parts(authorization: Option<&str>) -> Parts {
        let mut request = Request::get("/api/action-types");
        if let Some(value) = authorization {
            request = request.header("authorization", value);
        }
        request.body(()).unwrap().into_parts().0
    }

    /// A well-formed access token whose `exp` is against the real clock, which
    /// is the one `authorize` reads. Everything below `verified` takes its clock
    /// as an argument and uses [`testkey::NOW`] instead.
    fn current_token() -> String {
        testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","client_id":"{}","token_use":"access","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                unix_time() + 3600,
            ),
        )
    }

    #[test]
    fn refuses_a_request_with_no_authorization_header() {
        assert!(matches!(
            authorize(&parts(None), &verifier()),
            Authorization::Refused
        ));
    }

    #[test]
    fn refuses_an_empty_authorization_header() {
        assert!(matches!(
            authorize(&parts(Some("Bearer ")), &verifier()),
            Authorization::Refused
        ));
    }

    /// The prefix is optional at the deployed authorizer's identity source, so a
    /// bare token is a caller here too.
    #[test]
    fn accepts_a_token_without_the_bearer_prefix() {
        assert!(matches!(
            authorize(&parts(Some(&current_token())), &verifier()),
            Authorization::Allowed(_)
        ));
    }

    // --- the verdict --------------------------------------------------------
    //
    // Every case below signs with the committed fixture key and is checked
    // against a fixed clock, so nothing here depends on a real pool, on the
    // network, or on the date the tests are run — DR-0022.

    fn verifier() -> Verifier {
        Verifier {
            verification: Verification {
                issuer: testkey::ISSUER.to_owned(),
                audience: testkey::AUDIENCE.to_owned(),
            },
            keys: Keys::parse(testkey::JWKS.as_bytes()).unwrap(),
        }
    }

    /// A real Cognito **access** token's shape: the app client id in
    /// `client_id`, and no `aud` at all.
    fn access_token() -> String {
        testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","client_id":"{}","token_use":"access","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                testkey::NOW + 3600,
            ),
        )
    }

    /// A real Cognito **id** token's shape: the app client id in `aud`, and no
    /// `client_id`.
    fn id_token() -> String {
        testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","aud":"{}","token_use":"id","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                testkey::NOW + 3600,
            ),
        )
    }

    fn verify(token: &str) -> Result<Map<String, Value>, String> {
        verifier().verified(token, testkey::NOW)
    }

    #[test]
    fn an_access_token_from_the_pool_is_allowed() {
        let claims = verify(&access_token()).expect("a well-formed access token verifies");

        assert_eq!(
            claims.get("sub"),
            Some(&Value::String("abc-123".to_owned()))
        );
    }

    /// The other half of the audience rule. API Gateway accepts either, so this
    /// adapter has to as well — the point being that one checking only one of
    /// the two claims would be silently wrong in one direction.
    #[test]
    fn an_id_token_carrying_the_app_client_in_aud_is_allowed() {
        assert!(verify(&id_token()).is_ok());
    }

    /// `aud` as an array, which RFC 7519 permits and which a pool with more than
    /// one app client can produce.
    #[test]
    fn an_aud_array_containing_the_app_client_is_allowed() {
        let token = testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","aud":["other","{}"],"exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                testkey::NOW + 3600,
            ),
        );

        assert!(verify(&token).is_ok());
    }

    /// The check that makes this adapter worth running: everything else about
    /// the token is well formed and every other check passes.
    #[test]
    fn a_token_whose_signature_was_tampered_with_is_refused() {
        let token = access_token();
        let (signing_input, signature) = token.rsplit_once('.').unwrap();

        // One byte of the signature, not of the payload: the claims are
        // untouched and every other check would still pass.
        let mut bytes = URL_SAFE_NO_PAD.decode(signature).unwrap();
        bytes[0] ^= 0x01;
        let tampered = format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(&bytes));

        let error = verify(&tampered).unwrap_err();
        assert!(error.contains("signature"), "{error}");
    }

    /// Changing a claim invalidates the signature, so the payload cannot be
    /// edited without the key either — which is what makes the claims the
    /// service acts on trustworthy.
    #[test]
    fn a_token_whose_subject_was_swapped_is_refused() {
        let token = access_token();
        let (header, rest) = token.split_once('.').unwrap();
        let (_, signature) = rest.split_once('.').unwrap();

        let forged = URL_SAFE_NO_PAD.encode(format!(
            r#"{{"sub":"attacker","iss":"{}","client_id":"{}","exp":{}}}"#,
            testkey::ISSUER,
            testkey::AUDIENCE,
            testkey::NOW + 3600,
        ));

        let error = verify(&format!("{header}.{forged}.{signature}")).unwrap_err();
        assert!(error.contains("signature"), "{error}");
    }

    #[test]
    fn an_expired_token_is_refused() {
        let token = testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","client_id":"{}","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                testkey::NOW - 1,
            ),
        );

        let error = verify(&token).unwrap_err();
        assert!(error.contains("expired"), "{error}");
    }

    /// `jwt_configuration.audience` naming the wrong app client, which is one of
    /// the three ways `infra/api/apigateway.tf` can be subtly wrong.
    #[test]
    fn a_token_from_another_app_client_is_refused() {
        let token = testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","client_id":"another-app-client","token_use":"access","exp":{}}}"#,
                testkey::ISSUER,
                testkey::NOW + 3600,
            ),
        );

        let error = verify(&token).unwrap_err();
        assert!(error.contains("app client"), "{error}");
        // The message names which kind of token arrived, because "you are
        // sending the id token" is the diagnosis this phase is for.
        assert!(error.contains("access"), "{error}");
    }

    /// `jwt_configuration.issuer` not being the pool's issuer URL exactly, which
    /// is another of the three.
    #[test]
    fn a_token_from_another_issuer_is_refused() {
        let token = testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            &format!(
                r#"{{"sub":"abc-123","iss":"{}/","client_id":"{}","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                testkey::NOW + 3600,
            ),
        );

        let error = verify(&token).unwrap_err();
        assert!(error.contains("`iss`"), "{error}");
    }

    /// The Work Log is explicit that a `kid` outside the set is a 401 and not a
    /// fetch loop. `Keys::verify` is where that is enforced; this is the path
    /// reaching it.
    #[test]
    fn a_token_signed_under_an_unknown_kid_is_refused() {
        let token = testkey::sign(
            r#"{"alg":"RS256","kid":"some-other-pool"}"#,
            &format!(
                r#"{{"sub":"abc-123","iss":"{}","client_id":"{}","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                testkey::NOW + 3600,
            ),
        );

        let error = verify(&token).unwrap_err();
        assert!(error.contains("`kid`"), "{error}");
    }

    /// `alg: none` with an empty signature is the attack this shape has, and it
    /// is refused before any key is looked for.
    #[test]
    fn a_token_asserting_no_algorithm_is_refused() {
        let unsigned = format!(
            "{}.{}.",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#),
            URL_SAFE_NO_PAD.encode(format!(
                r#"{{"sub":"attacker","iss":"{}","client_id":"{}","exp":{}}}"#,
                testkey::ISSUER,
                testkey::AUDIENCE,
                testkey::NOW + 3600,
            )),
        );

        let error = verify(&unsigned).unwrap_err();
        assert!(error.contains("RS256"), "{error}");
    }

    /// The whole path, header to claims, rather than `verified` on its own —
    /// which is also the only thing that runs `report`, whose line is what tells
    /// an access token from an id token in a terminal.
    #[test]
    fn a_good_token_reaches_the_service_through_authorize() {
        let verifier = verifier();
        let header = format!("Bearer {}", access_token());

        // The fixed clock does not reach `authorize`, which reads the real one,
        // so the token has to be valid now as well as at `testkey::NOW`.
        match authorize(
            &parts(Some(&format!("Bearer {}", current_token()))),
            &verifier,
        ) {
            Authorization::Allowed(claims) => assert_eq!(
                claims.get("sub"),
                Some(&Value::String("abc-123".to_owned()))
            ),
            Authorization::Refused => panic!("expected the token to verify"),
        }

        // And the fixed-clock one is refused through the same path, because its
        // `exp` is years in the past by the time anyone runs this.
        assert!(matches!(
            authorize(&parts(Some(&header)), &verifier),
            Authorization::Refused
        ));
    }

    /// Stated as a test so that it is a decision rather than a surprise: there
    /// is nothing to verify a bare name against, so the two-callers-one-curl-flag
    /// affordance is `just dev-api`'s and not this adapter's — DR-0024.
    #[test]
    fn a_bare_name_is_refused() {
        let error = verify("alice").unwrap_err();

        assert!(error.contains("not a JWT"), "{error}");
        assert!(error.contains("dev-api"), "{error}");
    }
}
