//! One RSA key, minted once and committed, so that the verification tests can
//! sign a token and then verify it.
//!
//! It is not a secret and is not treated as one. Nothing anywhere trusts it: it
//! exists so that a test can produce a token that is genuinely well formed —
//! same segments, same signing input, same RS256 signature as a Cognito token —
//! and then vary one thing about it. A pre-signed fixture would have been
//! smaller but would have fixed `exp` along with everything else, and half of
//! what is being checked is what happens when a claim is wrong.
//!
//! `verified` takes its clock as an argument for the same reason, so no test
//! here depends on the date it is run.
//!
//! Minted with what the devcontainer image has — no Python, and no `xxd`:
//!
//! ```text
//! # The private half. `genpkey -outform DER` writes PKCS#1 here, not PKCS#8,
//! # whatever the manual suggests, and `from_pkcs8` rejects it — hence the
//! # second command, which is not optional.
//! openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
//!     -outform DER -out pkcs1.der
//! openssl pkcs8 -topk8 -nocrypt -inform DER -in pkcs1.der \
//!     -outform DER -out testkey.der
//!
//! # The modulus, base64url and unpadded, which is `n` in the key set below.
//! openssl rsa -in testkey.der -inform DER -pubout -outform DER |
//!     openssl rsa -pubin -inform DER -noout -modulus |
//!     sed 's/^Modulus=//' | basenc --base16 -d | basenc --base64url -w0 |
//!     tr -d '='
//! ```

/// The private half, PKCS#8 DER, which is what
/// `aws_lc_rs::signature::RsaKeyPair::from_pkcs8` reads.
pub const PRIVATE_KEY: &[u8] = include_bytes!("testkey.der");

/// The `kid` the tests sign under, and the one the key set below publishes.
pub const KID: &str = "devgateway-test";

/// The public half, in the shape Cognito publishes at
/// `{issuer}/.well-known/jwks.json`. The second entry is an EC key that is
/// deliberately not RSA, so that the parser's skipping of one is exercised by
/// the same document every other test loads.
pub const JWKS: &str = r#"{
  "keys": [
    {
      "alg": "RS256",
      "e": "AQAB",
      "kid": "devgateway-test",
      "kty": "RSA",
      "n": "uHec18RptKdtVu2Kv3jdVXzlB5KWaXkThGteQMFv5L8nJGhl2yZZlXO_Iy1CLL9-E7YO1O7a8Q14sgThGAyeOQ_DAMhfKCzln9PmBt_SqsefyFp71YqPmUAsSbHlEAKZ9ueetjNa4oR-C2bu91gO2Ckz7zgdcEClTcxMWTmsV3x1_0EV6j5geoQka5W49dwCu1lT9NBvy3uLalRsxpqr02yyJ0Hb6dYR6Z0rIST3Ye6B-Mf5kFBKRLILYdwmJKbUWWkpS1cU0H4Ydta9o_gth3wvzhaWrZ-sRNoGNatd3xWufwCIAk8iA5LJ3FMh2mKqgtihRQPBzw0tgq1Llo87GQ",
      "use": "sig"
    },
    {
      "alg": "ES256",
      "crv": "P-256",
      "kid": "not-an-rsa-key",
      "kty": "EC",
      "use": "sig",
      "x": "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
      "y": "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0"
    }
  ]
}"#;

/// The issuer the tests configure and sign with. Shaped like the real one —
/// `https://cognito-idp.<region>.amazonaws.com/<pool id>` — because the check
/// against it is an exact string comparison and a shape that does not look like
/// the real one would hide nothing.
pub const ISSUER: &str = "https://cognito-idp.ap-northeast-1.amazonaws.com/ap-northeast-1_example";

/// The app client id the tests configure, standing in for
/// `local.app_client_id` in `infra/api/apigateway.tf`.
pub const AUDIENCE: &str = "1example23456789abcdefghij";

/// A fixed instant, so `exp` and `nbf` are compared against something that does
/// not move. 2026-01-01T00:00:00Z.
pub const NOW: u64 = 1_767_225_600;

/// Sign a JWT with the key above.
///
/// `header` and `payload` are JSON documents; the caller builds them so that a
/// test can put anything in either, including a `kid` that is not in the key
/// set.
pub fn sign(header: &str, payload: &str) -> String {
    use aws_lc_rs::rand::SystemRandom;
    use aws_lc_rs::signature::{self, RsaKeyPair};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let key = RsaKeyPair::from_pkcs8(PRIVATE_KEY).expect("the committed fixture is PKCS#8 DER");

    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload)
    );

    let mut signature = vec![0; key.public_modulus_len()];
    key.sign(
        &signature::RSA_PKCS1_SHA256,
        &SystemRandom::new(),
        signing_input.as_bytes(),
        &mut signature,
    )
    .expect("a 2048-bit key signs RS256");

    format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(&signature))
}
