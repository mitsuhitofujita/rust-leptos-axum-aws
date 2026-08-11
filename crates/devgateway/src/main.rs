//! A stand-in for the deployed API's edge, for use on a developer's machine.
//!
//! Between the browser and `crates/server` in a deployed request sit an API
//! Gateway HTTP API and the Lambda Web Adapter. Five of their behaviours are
//! decided in `infra/api/apigateway.tf` and are unobservable locally: the route
//! table, the preflight answered ahead of the authorizer (DR-0009), the 401 for
//! a request with no token (DR-0010), the request context arriving as a header
//! (DR-0017), and every claim in it being a string.
//!
//! This binary reproduces those five in front of the unmodified service. It is
//! not a general API Gateway emulator, and it is deliberately outside
//! `crates/server`: in the deployment it is outside, and DR-0017 rests on the
//! service being an ordinary axum binary that knows nothing about Lambda —
//! DR-0021.
//!
//! Its `cognito` mode adds the sixth behaviour, which is the authorizer's actual
//! verdict: the token is verified against the pool's published keys and against
//! the `issuer` and `audience` in `jwt_configuration`, so that a wrong one is
//! visible before an apply — DR-0022.
//!
//! It ships nothing. Like `crates/icongen`, it is a workspace member that exists
//! for the people working on the project.

mod authorizer;
mod config;
mod context;
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
use crate::config::{Config, Mode};
use crate::edge::Outcome;
use crate::jwks::Keys;
use crate::proxy::Forwarder;

struct Edge {
    config: Config,
    /// `Some` in `cognito` mode. Built before the listener binds, so a request
    /// never waits on a fetch and a `kid` that is not in the set is a refusal
    /// rather than a second one.
    verifier: Option<Verifier>,
    forwarder: Forwarder,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::from_environment()?;

    // Before the listener, deliberately. A pool that cannot be reached is a
    // reason to stop with the reason on screen, not to accept connections and
    // refuse every one of them for a cause nobody can see.
    let verifier = match config.verification.take() {
        Some(verification) => Some(Verifier {
            keys: Keys::fetch(&verification.issuer).await?,
            verification,
        }),
        None => None,
    };

    let listener = tokio::net::TcpListener::bind(&config.address).await?;
    announce(&config, verifier.as_ref(), listener.local_addr()?);

    let edge = Arc::new(Edge {
        config,
        verifier,
        forwarder: proxy::forwarder(),
    });

    // One fallback rather than a route table: the whole point is to answer 404
    // where the HTTP API answers 404, and a router would answer 405 for a method
    // it has no handler for. edge::decide is the route table.
    let app = Router::new().fallback(handle).with_state(edge);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle(State(edge): State<Arc<Edge>>, request: Request) -> Response {
    let (mut parts, body) = request.into_parts();

    match edge::decide(&edge.config, edge.verifier.as_ref(), &mut parts) {
        Outcome::Answer(response) => response,
        Outcome::Forward(allow_origin) => {
            proxy::forward(
                &edge.forwarder,
                &edge.config.upstream,
                parts,
                body,
                allow_origin,
            )
            .await
        }
    }
}

fn announce(config: &Config, verifier: Option<&Verifier>, address: std::net::SocketAddr) {
    println!("devgateway listening on http://{address}");
    println!("forwarding to {}", config.upstream);

    match config.mode {
        Mode::Local => println!(
            "mode local: /api needs an Authorization header, \
             and the request context is written here"
        ),
        Mode::Passthrough => println!(
            "mode passthrough: nothing is reproduced and nothing is discarded, \
             which is what `just dev-api` alone already does"
        ),
        // The two configured values are echoed because they are what is being
        // checked. A wrong `jwt_configuration` in infra/api/apigateway.tf is
        // this mode's whole subject, and reading it back here is the cheapest
        // half of the check.
        Mode::Cognito => {
            let Some(verifier) = verifier else {
                unreachable!("cognito mode carries a verifier")
            };

            println!(
                "mode cognito: tokens are verified against {} key(s) from the pool",
                verifier.keys.len()
            );
            println!("  issuer   {}", verifier.verification.issuer);
            println!("  audience {}", verifier.verification.audience);
            println!(
                "  `Bearer alice` does not work here — there is nothing to \
                 verify a bare name against. Use `local` mode for two callers."
            );
        }
    }
}
