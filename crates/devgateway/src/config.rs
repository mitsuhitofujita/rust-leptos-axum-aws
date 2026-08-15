//! What the adapter reads from its environment, and what every unset value
//! means.
//!
//! Two of the three have a default, the way DR-0008 defaults the SPA's
//! configuration and DR-0018 the service's: an unset value means something
//! workable rather than something broken. The pair below is the exception, and
//! it is the whole reason this crate exists — see [`verification`].

use std::env;

/// Where the adapter listens. The service keeps 3000 — it binds it as a
/// constant and is not modified for this.
const ADDRESS: &str = "127.0.0.1:3001";

/// The unmodified service, reached over plain HTTP on loopback.
const UPSTREAM: &str = "http://127.0.0.1:3000";

/// What the deployed authorizer's `jwt_configuration` holds, which is the whole
/// of what this adapter needs to be told.
///
/// Both mirror `infra/api/apigateway.tf`, and `just dev-gateway` resolves both
/// from the same SSM parameters the `api` layer reads them from — so the adapter
/// is checking the deployed configuration and not a transcription of it.
pub struct Verification {
    /// `jwt_configuration.issuer`. Compared against the token's `iss` exactly.
    pub issuer: String,
    /// `jwt_configuration.audience`, which is the app client id.
    pub audience: String,
}

pub struct Config {
    pub address: String,
    pub upstream: String,
    pub verification: Verification,
}

impl Config {
    pub fn from_environment() -> Result<Self, String> {
        Ok(Self {
            address: var("DEVGATEWAY_ADDRESS", ADDRESS),
            upstream: var("DEVGATEWAY_UPSTREAM", UPSTREAM)
                .trim_end_matches('/')
                .to_owned(),
            verification: verification()?,
        })
    }
}

fn var(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

/// Neither of these has a default, which is where this crate departs from the
/// rule at the top of the file.
///
/// A defaulted issuer would verify against the wrong pool and refuse every real
/// token; a defaulted audience would accept tokens the deployed authorizer
/// refuses. Both failures look exactly like the misconfiguration this adapter
/// exists to catch, so an unset value is refused where the reason is still
/// obvious — DR-0022.
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
        .ok_or_else(|| format!("{name} is unset. `just dev-gateway` resolves it from SSM."))
}
