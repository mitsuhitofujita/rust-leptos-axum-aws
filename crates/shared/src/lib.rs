//! Types exchanged between the frontend (`app`) and the API (`server`).
//!
//! This crate must stay free of platform-specific dependencies: it is compiled
//! both for `wasm32-unknown-unknown` and for the server's native target. That is
//! why an instant is a string here rather than a date type — no crate that
//! parses one is worth adding to both targets for a value the browser only
//! formats.

use serde::{Deserialize, Serialize};

pub mod icon_names;

/// A kind of action, registered before any of its actions are recorded. It
/// supplies both the displayed name and the unit of the numeric value, so
/// `Running` and `km` are what make the record `Running — 5.2 km`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionType {
    pub id: String,
    pub name: String,
    pub unit: String,
    /// Which glyph the frontend draws for this type — a canonical kebab-case
    /// Lucide name such as `person-standing`, not markup. `shared` is compiled
    /// for the server too and must not carry a view, so this side of the wire
    /// holds only [`icon_names`]; a name the frontend does not know falls back
    /// to a generic glyph rather than rendering nothing (DR-0014).
    pub icon: String,
}

/// What creating an action type takes: everything [`ActionType`] has except the
/// identifier, which the service assigns.
///
/// A separate type rather than an [`ActionType`] with an ignored `id`, so that
/// a client cannot appear to choose one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewActionType {
    pub name: String,
    pub unit: String,
    pub icon: String,
}

/// One completed action: a registered type paired with a numeric value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub id: String,
    pub action_type: ActionType,
    /// One field for every unit, so `5.2 km` and `6200 steps` are the same
    /// shape.
    pub value: f64,
    /// RFC 3339. The browser converts it to local time for display.
    pub recorded_at: String,
}

/// The recent ten-day window, oldest day first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentSummary {
    /// Every action in the window — the sum of `daily`, not the length of
    /// [`Dashboard::recent`]. The window and the recent list are separate
    /// limits.
    pub total: u32,
    /// Exactly ten counts, one per day.
    pub daily: Vec<u32>,
}

/// The payload of `GET /api/dashboard`.
///
/// One response rather than two, because the dashboard is the unit it serves:
/// the screen has one loading state and one error state either way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dashboard {
    pub summary: RecentSummary,
    /// The ten latest records, newest first.
    pub recent: Vec<ActionRecord>,
}
