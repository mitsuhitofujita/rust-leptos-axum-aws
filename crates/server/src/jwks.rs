//! The user pool's signing keys, and the one thing done with them.
//!
//! Cognito publishes them at `{issuer}/.well-known/jwks.json`, which is the same
//! document `aws_apigatewayv2_authorizer.cognito` read before DR-0028 retired
//! it, and the reason the fetch needs no AWS credentials: the endpoint is
//! public. Credentials are needed to learn the issuer, not to fetch this.
//!
//! **Fetched once, before the listener binds, and held for the process's
//! lifetime.** Two things follow from that and both are deliberate. A `kid`
//! that is not in the set is a refusal and never a second fetch, so a token
//! signed with anything else cannot turn into a request to Cognito per
//! attempt. And a pool whose keys have rotated since the process started
//! means a Lambda cold start picks them up fresh — the cost is paid once per
//! execution environment, not per request.
//!
//! Fetching eagerly is also what keeps [`crate::identity::Auth::from_environment`]
//! synchronous everywhere but its own construction: by the time a request
//! arrives there is nothing left to await.

use std::collections::HashMap;

use aws_lc_rs::signature::{RSA_PKCS1_2048_8192_SHA256, RsaPublicKeyComponents};
use axum::body::Body;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::Deserialize;

/// A JWKS document is a few kilobytes. The cap is here so that a wrong URL
/// answering with something enormous is an error rather than memory.
const MAX_DOCUMENT: usize = 256 * 1024;

/// One RSA public key, with its two components already decoded — which is the
/// form `aws-lc-rs` wants and the reason nothing here parses ASN.1.
struct Rsa {
    n: Vec<u8>,
    e: Vec<u8>,
}

/// The key set, by `kid`.
pub struct Keys(HashMap<String, Rsa>);

impl Keys {
    /// Read a JWKS document.
    ///
    /// Entries that are not RS256 RSA keys are skipped rather than refused: a
    /// pool is free to publish others, and this only ever validates RS256. A
    /// document from which nothing survives is an error, because it is
    /// indistinguishable in effect from a wrong URL and would otherwise
    /// surface much later as an unexplained 401.
    pub fn parse(document: &[u8]) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Set {
            keys: Vec<Key>,
        }

        #[derive(Deserialize)]
        struct Key {
            kty: String,
            kid: Option<String>,
            alg: Option<String>,
            n: Option<String>,
            e: Option<String>,
        }

        let set: Set = serde_json::from_slice(document)
            .map_err(|error| format!("the key set is not a JWKS document: {error}"))?;

        let mut keys = HashMap::new();

        for key in set.keys {
            if key.kty != "RSA" || key.alg.as_deref().is_some_and(|alg| alg != "RS256") {
                continue;
            }

            let (Some(kid), Some(n), Some(e)) = (key.kid, key.n, key.e) else {
                continue;
            };

            let (Ok(n), Ok(e)) = (URL_SAFE_NO_PAD.decode(&n), URL_SAFE_NO_PAD.decode(&e)) else {
                return Err(format!("the components of key `{kid}` are not base64url"));
            };

            keys.insert(kid, Rsa { n, e });
        }

        if keys.is_empty() {
            return Err("the key set holds no RS256 RSA key".to_owned());
        }

        Ok(Self(keys))
    }

    /// Fetch the set the pool publishes.
    pub async fn fetch(issuer: &str) -> Result<Self, String> {
        let url = format!("{}/.well-known/jwks.json", issuer.trim_end_matches('/'));
        let uri = url
            .parse()
            .map_err(|error| format!("{url} is not a URI: {error}"))?;

        // The image's own trust anchors — /etc/ssl/certs, the same ones curl and
        // the AWS CLI use — rather than a root store bundled into this binary,
        // which would be a second thing to keep current.
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|error| format!("the system trust anchors are unreadable: {error}"))?
            .https_only()
            .enable_http1()
            .build();

        let client: Client<_, Body> = Client::builder(TokioExecutor::new()).build(connector);

        let response = client
            .get(uri)
            .await
            .map_err(|error| format!("{url} did not answer: {error}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("{url} answered {status}"));
        }

        let document = axum::body::to_bytes(Body::new(response.into_body()), MAX_DOCUMENT)
            .await
            .map_err(|error| format!("{url} answered a body that could not be read: {error}"))?;

        Self::parse(&document).map_err(|reason| format!("{url}: {reason}"))
    }

    /// How many keys the set holds, for the startup announcement.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check an RS256 signature against the key the token names.
    ///
    /// `signing_input` is the token's first two segments and the dot between
    /// them, exactly as they arrived — re-encoding the decoded header or payload
    /// would produce different bytes and fail every time.
    pub fn verify(
        &self,
        kid: &str,
        signing_input: &[u8],
        signature: &[u8],
    ) -> Result<(), &'static str> {
        let key = self.0.get(kid).ok_or(
            "the token names a `kid` this pool's key set does not hold; \
             it was signed by something else, or the pool has rotated its keys \
             since this process started",
        )?;

        RsaPublicKeyComponents {
            n: &key.n,
            e: &key.e,
        }
        .verify(&RSA_PKCS1_2048_8192_SHA256, signing_input, signature)
        .map_err(|_| "the signature is not the one this pool's key would have produced")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkey;

    fn keys() -> Keys {
        Keys::parse(testkey::JWKS.as_bytes()).expect("the committed fixture parses")
    }

    /// `Keys` has no `Debug`, deliberately — nothing wants a key set in a panic
    /// message — so the error cases unwrap by hand.
    fn refused(document: &[u8]) -> String {
        match Keys::parse(document) {
            Err(reason) => reason,
            Ok(_) => panic!("expected the key set to be refused"),
        }
    }

    /// The fixture publishes an EC key beside the RSA one. Something that only
    /// validates RS256 has nothing to do with it, and a parser that refused the
    /// whole document over it would refuse a pool that is perfectly valid.
    #[test]
    fn a_key_that_is_not_an_rs256_rsa_key_is_skipped() {
        let keys = keys();

        assert_eq!(keys.len(), 1);
        assert!(keys.0.contains_key(testkey::KID));
        assert!(!keys.0.contains_key("not-an-rsa-key"));
    }

    /// Indistinguishable in effect from a wrong URL, so it is refused where the
    /// reason is still known rather than surfacing later as a bare 401.
    #[test]
    fn a_set_with_no_usable_key_is_refused() {
        let error = refused(br#"{"keys":[{"kty":"EC","kid":"a"}]}"#);

        assert!(error.contains("no RS256 RSA key"), "{error}");
    }

    #[test]
    fn something_that_is_not_a_jwks_document_is_refused() {
        let error = refused(b"<html>404</html>");

        assert!(error.contains("not a JWKS document"), "{error}");
    }

    /// A token signed by anything else must not turn into one request to
    /// Cognito per attempt — it is a refusal and not a refetch.
    #[test]
    fn an_unknown_kid_is_an_error_rather_than_a_fetch() {
        let error = keys()
            .verify("some-other-kid", b"a.b", b"signature")
            .unwrap_err();

        assert!(error.contains("does not hold"), "{error}");
    }

    #[test]
    fn a_signature_from_the_fixture_key_verifies() {
        let token = testkey::sign(
            &format!(r#"{{"alg":"RS256","kid":"{}"}}"#, testkey::KID),
            r#"{"sub":"abc-123"}"#,
        );
        let (signing_input, signature) = token.rsplit_once('.').unwrap();

        assert!(
            keys()
                .verify(
                    testkey::KID,
                    signing_input.as_bytes(),
                    &URL_SAFE_NO_PAD.decode(signature).unwrap(),
                )
                .is_ok()
        );
    }
}
