//! `/api/actions`: listing, recording, reading, updating and deleting one
//! owner's action records.
//!
//! Mirrors [`crate::action_types`] in shape: the owner comes from
//! [`crate::identity`] and nowhere else, and a `Failure` carries the reason a
//! visitor sees. What differs is what may change after creation — DR-0016
//! fixes a record's type and its copied display attributes, so [`update`]
//! takes only a value, not a full [`NewActionRecord`].

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use shared::{ActionRecord, NewActionRecord, UpdateActionRecord};

use crate::identity::Owner;
use crate::store::{Store, StoreError};

pub async fn list(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
) -> Result<Json<Vec<ActionRecord>>, Failure> {
    Ok(Json(store.list_action_records(&owner).await?))
}

pub async fn create(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Json(new): Json<NewActionRecord>,
) -> Result<(StatusCode, Json<ActionRecord>), Failure> {
    let new = validate(new)?;

    store
        .create_action_record(&owner, new)
        .await?
        .map(|created| (StatusCode::CREATED, Json(created)))
        .ok_or_else(Failure::unknown_type)
}

pub async fn get_one(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Path(id): Path<String>,
) -> Result<Json<ActionRecord>, Failure> {
    store
        .get_action_record(&owner, &id)
        .await?
        .map(Json)
        .ok_or_else(Failure::not_found)
}

pub async fn update(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Path(id): Path<String>,
    Json(new): Json<UpdateActionRecord>,
) -> Result<Json<ActionRecord>, Failure> {
    let new = validate_value(new)?;

    store
        .update_action_record(&owner, &id, new)
        .await?
        .map(Json)
        .ok_or_else(Failure::not_found)
}

pub async fn delete(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    store.delete_action_record(&owner, &id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// What may be recorded. `type_id` is checked against the caller's own
/// partition by the store, not here — a `type_id` that names nothing is
/// [`Failure::unknown_type`], answered after the store has actually looked,
/// the same way a request is never accepted on the strength of an id that
/// merely looks well-formed.
fn validate(new: NewActionRecord) -> Result<NewActionRecord, Failure> {
    if new.type_id.trim().is_empty() {
        return Err(Failure::rejected("An action type is required."));
    }

    Ok(NewActionRecord {
        type_id: new.type_id,
        value: validate_value(UpdateActionRecord { value: new.value })?.value,
    })
}

fn validate_value(new: UpdateActionRecord) -> Result<UpdateActionRecord, Failure> {
    if !new.value.is_finite() {
        return Err(Failure::rejected("A numeric value is required."));
    }

    Ok(new)
}

/// Why a request produced no value. Mirrors [`crate::action_types::Failure`]
/// with this module's own messages rather than sharing the type, matching
/// the existing precedent of one `Failure` per module.
#[derive(Debug)]
pub enum Failure {
    /// The request was understood and refused. The visitor can fix it.
    Rejected(String),
    /// The id in the URL named nothing in the caller's own partition.
    NotFound(String),
    /// The store did not answer. The visitor cannot.
    Unavailable(String),
}

impl Failure {
    fn rejected(reason: &str) -> Self {
        Self::Rejected(reason.to_owned())
    }

    fn not_found() -> Self {
        Self::NotFound("That action could not be found.".to_owned())
    }

    /// `type_id` is a body field naming another entity, not the URL's own
    /// addressed resource — a bad one is a rejected request, not a `404`,
    /// the same distinction a bad `icon` name gets in `action_types.rs`.
    fn unknown_type() -> Self {
        Self::Rejected("That action type could not be found.".to_owned())
    }
}

impl From<StoreError> for Failure {
    fn from(error: StoreError) -> Self {
        Self::Unavailable(error.to_string())
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        match self {
            Self::Rejected(reason) => (StatusCode::BAD_REQUEST, reason).into_response(),
            Self::NotFound(reason) => (StatusCode::NOT_FOUND, reason).into_response(),
            Self::Unavailable(reason) => {
                // The reason reaches the log, not the visitor: it is about the
                // store and there is nothing in it they could act on.
                eprintln!("store unavailable: {reason}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong on our side. Please try again.",
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejection(new: NewActionRecord) -> String {
        match validate(new) {
            Err(Failure::Rejected(reason)) => reason,
            Err(Failure::NotFound(reason)) => panic!("not found: {reason}"),
            Err(Failure::Unavailable(reason)) => panic!("unavailable: {reason}"),
            Ok(_) => panic!("accepted"),
        }
    }

    #[test]
    fn refuses_an_empty_type_id() {
        assert_eq!(
            rejection(NewActionRecord {
                type_id: "   ".to_owned(),
                value: 5.2,
            }),
            "An action type is required."
        );
    }

    #[test]
    fn refuses_a_non_finite_value() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                rejection(NewActionRecord {
                    type_id: "01JZZZZZZZZZZZZZZZZZZZZZZZ".to_owned(),
                    value,
                }),
                "A numeric value is required."
            );
        }
    }

    #[test]
    fn accepts_an_ordinary_value() {
        assert!(
            validate(NewActionRecord {
                type_id: "01JZZZZZZZZZZZZZZZZZZZZZZZ".to_owned(),
                value: 5.2,
            })
            .is_ok()
        );
    }
}
