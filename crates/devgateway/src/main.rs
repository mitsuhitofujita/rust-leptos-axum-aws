//! The deployed authorizer, in front of the unmodified service, on a
//! developer's machine.
//!
//! It answers one question — would `aws_apigatewayv2_authorizer.cognito` accept
//! this token? — by verifying it against the pool's published keys and against
//! the `issuer` and `audience` in `jwt_configuration`, so that a wrong one is
//! visible before an apply rather than as a 401 afterwards (DR-0022). What it
//! accepts, it converts into the `AuthContext` the service reads (DR-0024,
//! DR-0025), and forwards; what it does not, it refuses.
//!
//! It reproduces nothing else about API Gateway. The route table, the preflight
//! and the rest were reproduced here once and retracted, because each was a
//! second telling of a specification AWS holds and this repository cannot check
//! — DR-0023. One consequence is visible immediately: every path needs a token
//! here, including `/health`, which the deployment routes outside the
//! authorizer.
//!
//! It is deliberately outside `crates/server`: in the deployment the component
//! it stands in for is outside, and the service is an ordinary axum binary that
//! knows nothing about Lambda — DR-0024.
//!
//! It ships nothing. Like `crates/icongen`, it is a workspace member that exists
//! for the people working on the project.

mod authorizer;
mod config;
mod edge;
mod jwks;
mod proxy;
#[cfg(test)]
mod testkey;

use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::response::Response;

use crate::authorizer::Verifier;
use crate::config::Config;
use crate::edge::Outcome;
use crate::jwks::Keys;
use crate::proxy::Forwarder;

struct Edge {
    upstream: String,
    /// Built before the listener binds, so a request never waits on a fetch and
    /// a `kid` that is not in the set is a refusal rather than a second one.
    verifier: Verifier,
    forwarder: Forwarder,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_environment()?;

    // Before the listener, deliberately. A pool that cannot be reached is a
    // reason to stop with the reason on screen, not to accept connections and
    // refuse every one of them for a cause nobody can see.
    let verifier = Verifier {
        keys: Keys::fetch(&config.verification.issuer).await?,
        verification: config.verification,
    };

    let listener = tokio::net::TcpListener::bind(&config.address).await?;
    announce(&config.upstream, &verifier, listener.local_addr()?);

    let edge = Arc::new(Edge {
        upstream: config.upstream,
        verifier,
        forwarder: proxy::forwarder(),
    });

    // One fallback rather than a router, because there is nothing to route: the
    // same decision is made about every request whatever its path.
    let app = Router::new().fallback(handle).with_state(edge);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle(State(edge): State<Arc<Edge>>, request: Request) -> Response {
    let (mut parts, body) = request.into_parts();

    match edge::decide(&edge.verifier, &mut parts) {
        Outcome::Answer(response) => response,
        Outcome::Forward => proxy::forward(&edge.forwarder, &edge.upstream, parts, body).await,
    }
}

/// The two configured values are echoed because they are what is being checked.
/// A wrong `jwt_configuration` in infra/api/apigateway.tf is this adapter's
/// whole subject, and reading it back here is the cheapest half of the check.
fn announce(upstream: &str, verifier: &Verifier, address: std::net::SocketAddr) {
    println!("devgateway listening on http://{address}");
    println!("forwarding to {upstream}");
    println!(
        "tokens are verified against {} key(s) from the pool",
        verifier.keys.len()
    );
    println!("  issuer   {}", verifier.verification.issuer);
    println!("  audience {}", verifier.verification.audience);
    println!(
        "  every path needs a token, `/health` included, and `Bearer alice` is \
         not one. Two callers without a token is `just dev-api`."
    );
}
