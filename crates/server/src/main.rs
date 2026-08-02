//! Minimal axum API.
//!
//! This exists so the frontend has a real endpoint to fetch from. It carries no
//! domain logic yet.
//!
//! No CORS layer is configured: in development the browser talks to trunk,
//! which proxies `/api` here, so every request is same-origin. Production
//! serves the SPA from a different origin and will need CORS — see DR-0001.

use axum::{Json, Router, routing::get};
use shared::Greeting;

const ADDRESS: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/greeting", get(greeting));

    let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
    println!("API listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn greeting() -> Json<Greeting> {
    Json(Greeting::new("Hello from axum."))
}
