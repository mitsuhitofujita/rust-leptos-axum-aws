//! The dashboard endpoint, still answering from fixed values.
//!
//! Nothing here reads the table. The boundary types, the URL and the fetch path
//! are the real ones, so only the body of [`dashboard`] changes when the
//! dashboard is given data — which is a separate piece of work from the one
//! that gave the service a store at all.

use axum::Json;
use shared::{ActionRecord, ActionType, Dashboard, RecentSummary};

use crate::identity::Owner;

/// `Owner` is unused today because the values below are fixed, but the
/// extractor is what gates the route — a handler that does not name it is
/// reachable by anyone with no token, since gating moved from the route
/// table to the handler's own signature (DR-0028).
pub async fn dashboard(Owner(_owner): Owner) -> Json<Dashboard> {
    // The counts are per day, oldest first, and their sum is the total the
    // summary card reports. It is deliberately larger than the ten records
    // below: the ten-day window and the recent list are separate limits.
    let daily = vec![2, 4, 3, 5, 4, 6, 5, 7, 6, 8];

    Json(Dashboard {
        summary: RecentSummary {
            total: daily.iter().sum(),
            daily,
        },
        // The icons are canonical Lucide names from the supported catalog, the
        // same values a real action type stores. Anything else would draw the
        // frontend's fallback glyph on every row (DR-0014).
        recent: vec![
            record("Running", "km", "footprints", 5.2, "2026-08-08T07:12:00Z"),
            record("Water", "ml", "droplets", 450.0, "2026-08-08T09:40:00Z"),
            record(
                "Reading",
                "pages",
                "book-open",
                24.0,
                "2026-08-07T20:25:00Z",
            ),
            record("Meditation", "min", "brain", 10.0, "2026-08-07T06:30:00Z"),
            record("Cycling", "km", "bike", 12.4, "2026-08-05T17:55:00Z"),
            record(
                "Strength training",
                "reps",
                "dumbbell",
                30.0,
                "2026-08-05T07:10:00Z",
            ),
            record(
                "Study",
                "min",
                "graduation-cap",
                45.0,
                "2026-08-04T20:00:00Z",
            ),
            record(
                "Walking",
                "steps",
                "person-standing",
                6200.0,
                "2026-08-04T12:35:00Z",
            ),
            record("Sleep", "hours", "bed", 7.5, "2026-08-04T07:20:00Z"),
            record(
                "Stretching",
                "min",
                "activity",
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
