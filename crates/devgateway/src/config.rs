//! What the stand-in reads from its environment, and what every unset value
//! means.
//!
//! Every one has a default, so `cargo run -p devgateway` with nothing set is the
//! useful configuration. That is the same choice DR-0008 makes for the SPA and
//! DR-0018 for the service: an unset value means something workable rather than
//! something broken.

use std::env;

/// Where the stand-in listens. The service keeps 3000 — it binds it as a
/// constant and is not modified for this.
const ADDRESS: &str = "127.0.0.1:3001";

/// The unmodified service, reached over plain HTTP on loopback.
const UPSTREAM: &str = "http://127.0.0.1:3000";

/// Mirrors `cors_configuration.allow_origins` in `infra/api/apigateway.tf`,
/// which in the deployment is the CloudFront domain and here is the trunk dev
/// server.
const ALLOW_ORIGIN: &str = "http://localhost:8080";

/// Which of API Gateway's behaviours is being reproduced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// The edge is reproduced: routes, preflight, the authorizer, and a request
    /// context this process writes. The authorizer decodes a token without
    /// verifying it.
    Local,
    /// `Local`, and the authorizer verifies the token the way the deployed one
    /// does — DR-0022. The only mode that says anything about whether
    /// `aws_apigatewayv2_authorizer.cognito` would have accepted it.
    Cognito,
    /// Nothing is reproduced. The request is forwarded exactly as it arrived,
    /// which is what `just dev-api` alone already does.
    Passthrough,
}

/// What the deployed authorizer's `jwt_configuration` holds, which is the whole
/// of what `cognito` mode needs to be told.
///
/// Both mirror `infra/api/apigateway.tf`, and `just dev-gateway-cognito`
/// resolves both from the same SSM parameters the `api` layer reads them from —
/// so the mode is checking the deployed configuration and not a transcription
/// of it.
pub struct Verification {
    /// `jwt_configuration.issuer`. Compared against the token's `iss` exactly.
    pub issuer: String,
    /// `jwt_configuration.audience`, which is the app client id.
    pub audience: String,
}

pub struct Config {
    pub address: String,
    pub upstream: String,
    pub mode: Mode,
    pub allow_origin: String,
    /// `Some` exactly when the mode is `Cognito`.
    pub verification: Option<Verification>,
}

impl Config {
    pub fn from_environment() -> Result<Self, String> {
        let mode = mode()?;

        Ok(Self {
            address: var("DEVGATEWAY_ADDRESS", ADDRESS),
            upstream: var("DEVGATEWAY_UPSTREAM", UPSTREAM)
                .trim_end_matches('/')
                .to_owned(),
            mode,
            allow_origin: var("DEVGATEWAY_ALLOW_ORIGIN", ALLOW_ORIGIN),
            verification: match mode {
                Mode::Cognito => Some(verification()?),
                _ => None,
            },
        })
    }
}

fn var(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

/// An unrecognised mode is refused rather than defaulted. The modes differ in
/// whether a forged `x-amzn-request-context` reaches the service and in whether
/// a token is verified at all, so a typo silently selecting one of them is not a
/// failure to absorb.
fn mode() -> Result<Mode, String> {
    match var("DEVGATEWAY_MODE", "local").as_str() {
        "local" => Ok(Mode::Local),
        "cognito" => Ok(Mode::Cognito),
        "passthrough" => Ok(Mode::Passthrough),
        other => Err(format!(
            "DEVGATEWAY_MODE is `{other}`; it is `local`, `cognito` or `passthrough`"
        )),
    }
}

/// Neither of these has a default, which is the one place this crate departs
/// from the rule at the top of the file.
///
/// A defaulted issuer would verify against the wrong pool and refuse every real
/// token; a defaulted audience would accept tokens the deployed authorizer
/// refuses. Both failures look exactly like the misconfiguration this mode
/// exists to catch, so an unset value is refused where the reason is still
/// obvious.
fn verification() -> Result<Verification, String> {
    Ok(Verification {
        issuer: required("DEVGATEWAY_ISSUER")?,
        audience: required("DEVGATEWAY_AUDIENCE")?,
    })
}

fn required(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "DEVGATEWAY_MODE is `cognito` but {name} is unset. \
                 `just dev-gateway-cognito` resolves it from SSM."
            )
        })
}

#[cfg(test)]
impl Config {
    /// The defaults, without touching the environment — tests run in one
    /// process and share it.
    pub fn for_test(mode: Mode) -> Self {
        use crate::testkey;

        Self {
            address: ADDRESS.to_owned(),
            upstream: UPSTREAM.to_owned(),
            mode,
            allow_origin: ALLOW_ORIGIN.to_owned(),
            verification: (mode == Mode::Cognito).then(|| Verification {
                issuer: testkey::ISSUER.to_owned(),
                audience: testkey::AUDIENCE.to_owned(),
            }),
        }
    }
}
