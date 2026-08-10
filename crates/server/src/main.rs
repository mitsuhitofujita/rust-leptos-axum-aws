//! The axum API.
//!
//! Two endpoints answer from the store and one still answers from fixed values.
//! `/api/action-types` reads and writes the DynamoDB table
//! `docs/design/persistence.md` describes; `/api/dashboard` does not, and says
//! so in [`dashboard`].
//!
//! Every request under `/api` arrives having passed API Gateway's JWT
//! authorizer, which is the only thing enforcing anything (DR-0010). What the
//! service does with its conclusion is in [`identity`], and everything else here
//! trusts that and nothing else about the request.
//!
//! No CORS layer is configured: in development the browser talks to trunk, which
//! proxies `/api` here, so every request is same-origin. Production serves the
//! SPA from a different origin, and CORS is answered by the HTTP API rather than
//! here — see DR-0009.

mod action_types;
mod dashboard;
mod identity;
mod store;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::store::Store;

const ADDRESS: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Once, at startup: the SDK client is expensive to build and cheap to
    // share, and which store this is cannot change while the process runs.
    let store = Arc::new(Store::from_environment().await);
    println!("action types are stored in {}", store.describe());

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard::dashboard))
        .route(
            "/api/action-types",
            get(action_types::list).post(action_types::create),
        )
        .with_state(store);

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
