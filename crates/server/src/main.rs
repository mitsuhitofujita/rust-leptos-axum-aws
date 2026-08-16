//! The axum API.
//!
//! Two endpoints answer from the store and one still answers from fixed values.
//! `/api/action-types` reads and writes the DynamoDB table
//! `docs/design/persistence.md` describes; `/api/dashboard` does not, and says
//! so in [`dashboard`].
//!
//! Every request under `/api` names [`identity::Owner`] in its handler
//! signature, which is what decides who the caller is and whether they are
//! one at all — DR-0028. `/health` names it deliberately, since a probe has
//! no token.
//!
//! No CORS layer is configured: in development the browser talks to trunk, which
//! proxies `/api` here, so every request is same-origin. Production serves the
//! SPA from a different origin, and CORS is answered by the HTTP API rather than
//! here — see DR-0009.

mod action_types;
mod cognito;
mod dashboard;
mod identity;
mod jwks;
mod store;
#[cfg(test)]
mod testkey;

use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::routing::get;

use crate::identity::Auth;
use crate::store::Store;

const ADDRESS: &str = "127.0.0.1:3000";

#[derive(Clone)]
struct AppState {
    store: Arc<Store>,
    auth: Arc<Auth>,
}

impl FromRef<AppState> for Arc<Store> {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

impl FromRef<AppState> for Arc<Auth> {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Once, at startup: the SDK client is expensive to build and cheap to
    // share, and which store this is cannot change while the process runs.
    let store = Arc::new(Store::from_environment().await);
    println!("action types are stored in {}", store.describe());

    // Before the listener, deliberately — a pool that cannot be reached is a
    // reason to stop with the reason on screen, not to accept connections and
    // refuse every one of them for a cause nobody can see.
    let auth = Arc::new(Auth::from_environment().await?);
    println!("callers are authenticated by {}", auth.describe());

    let state = AppState { store, auth };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard::dashboard))
        .route(
            "/api/action-types",
            get(action_types::list).post(action_types::create),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
    println!("API listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// What the Lambda Web Adapter's readiness check asks for, and what a probe
/// gets. Deliberately unauthenticated, and it reveals nothing.
async fn health() -> &'static str {
    "ok"
}
