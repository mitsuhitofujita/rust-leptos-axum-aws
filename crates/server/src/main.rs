//! Minimal axum API.
//!
//! The dashboard endpoint answers with fixed values. Nothing is stored and
//! nothing is queried yet: the point of serving them from here rather than
//! embedding them in the frontend is that the boundary types, the URL and the
//! fetch path are already the real ones, so only the body of [`dashboard`]
//! changes when there is data to return.
//!
//! No CORS layer is configured: in development the browser talks to trunk,
//! which proxies `/api` here, so every request is same-origin. Production
//! serves the SPA from a different origin, and CORS is answered by the HTTP API
//! rather than here — see DR-0009.

use axum::{Json, Router, routing::get};
use shared::{ActionRecord, ActionType, Dashboard, RecentSummary};

const ADDRESS: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard));

    let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
    println!("API listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn dashboard() -> Json<Dashboard> {
    // The counts are per day, oldest first, and their sum is the total the
    // summary card reports. It is deliberately larger than the ten records
    // below: the ten-day window and the recent list are separate limits.
    let daily = vec![2, 4, 3, 5, 4, 6, 5, 7, 6, 8];

    Json(Dashboard {
        summary: RecentSummary {
            total: daily.iter().sum(),
            daily,
        },
        recent: vec![
            record("Running", "km", "running", 5.2, "2026-08-08T07:12:00Z"),
            record("Water", "ml", "water", 450.0, "2026-08-08T09:40:00Z"),
            record("Reading", "pages", "reading", 24.0, "2026-08-07T20:25:00Z"),
            record(
                "Meditation",
                "min",
                "meditation",
                10.0,
                "2026-08-07T06:30:00Z",
            ),
            record("Cycling", "km", "cycling", 12.4, "2026-08-05T17:55:00Z"),
            record(
                "Strength training",
                "reps",
                "strength",
                30.0,
                "2026-08-05T07:10:00Z",
            ),
            record("Study", "min", "study", 45.0, "2026-08-04T20:00:00Z"),
            record(
                "Walking",
                "steps",
                "walking",
                6200.0,
                "2026-08-04T12:35:00Z",
            ),
            record("Sleep", "hours", "sleep", 7.5, "2026-08-04T07:20:00Z"),
            record(
                "Stretching",
                "min",
                "stretching",
                15.0,
                "2026-08-03T21:10:00Z",
            ),
        ],
    })
}

/// One dummy record. The action type's id is its icon id, which is only true
/// while these values are hardcoded — real types will carry their own id.
fn record(name: &str, unit: &str, icon: &str, value: f64, recorded_at: &str) -> ActionRecord {
    ActionRecord {
        id: format!("{icon}-{recorded_at}"),
        action_type: ActionType {
            id: icon.to_owned(),
            name: name.to_owned(),
            unit: unit.to_owned(),
            icon: icon.to_owned(),
        },
        value,
        recorded_at: recorded_at.to_owned(),
    }
}
