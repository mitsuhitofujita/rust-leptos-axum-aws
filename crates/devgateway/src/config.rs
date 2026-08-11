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

/// Which of API Gateway's two behaviours is being reproduced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// The edge is reproduced: routes, preflight, the authorizer, and a request
    /// context this process writes.
    Local,
    /// Nothing is reproduced. The request is forwarded exactly as it arrived,
    /// which is what `just dev-api` alone already does.
    Passthrough,
}

pub struct Config {
    pub address: String,
    pub upstream: String,
    pub mode: Mode,
    pub allow_origin: String,
}

impl Config {
    pub fn from_environment() -> Result<Self, String> {
        Ok(Self {
            address: var("DEVGATEWAY_ADDRESS", ADDRESS),
            upstream: var("DEVGATEWAY_UPSTREAM", UPSTREAM)
                .trim_end_matches('/')
                .to_owned(),
            mode: mode()?,
            allow_origin: var("DEVGATEWAY_ALLOW_ORIGIN", ALLOW_ORIGIN),
        })
    }
}

fn var(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

/// An unrecognised mode is refused rather than defaulted. The two modes differ
/// in whether a forged `x-amzn-request-context` reaches the service, so a typo
/// silently selecting one of them is not a failure to absorb.
fn mode() -> Result<Mode, String> {
    match var("DEVGATEWAY_MODE", "local").as_str() {
        "local" => Ok(Mode::Local),
        "passthrough" => Ok(Mode::Passthrough),
        other => Err(format!(
            "DEVGATEWAY_MODE is `{other}`; it is `local` or `passthrough`"
        )),
    }
}

#[cfg(test)]
impl Config {
    /// The defaults, without touching the environment — tests run in one
    /// process and share it.
    pub fn for_test(mode: Mode) -> Self {
        Self {
            address: ADDRESS.to_owned(),
            upstream: UPSTREAM.to_owned(),
            mode,
            allow_origin: ALLOW_ORIGIN.to_owned(),
        }
    }
}
