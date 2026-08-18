//! `/api/dashboard`: the authenticated overview's one read.
//!
//! Two separate `Store` queries, not one — `docs/design/persistence.md`'s
//! recent-list and ten-day-summary limits are independent, and the list is
//! not required to contain one record per chart bar.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use shared::Dashboard;

use crate::identity::Owner;
use crate::store::{Store, StoreError};

pub async fn dashboard(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
) -> Result<Json<Dashboard>, Failure> {
    let summary = store.recent_summary(&owner).await?;
    let recent = store.recent_action_records(&owner).await?;

    Ok(Json(Dashboard { summary, recent }))
}

/// The store did not answer. This handler validates nothing and locates
/// nothing by id, so — unlike `action_types::Failure`/`actions::Failure` —
/// it has no other way to fail.
#[derive(Debug)]
pub struct Failure(String);

impl From<StoreError> for Failure {
    fn from(error: StoreError) -> Self {
        Self(error.to_string())
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        // The reason reaches the log, not the visitor: it is about the store
        // and there is nothing in it they could act on.
        eprintln!("store unavailable: {}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong on our side. Please try again.",
        )
            .into_response()
    }
}
