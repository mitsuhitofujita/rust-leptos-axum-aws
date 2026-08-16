//! `/api/action-types`: listing, creating, reading, updating and deleting what
//! an owner has registered.
//!
//! The owner is never a request parameter. It comes from [`crate::identity`],
//! which reads it from the claims the authorizer already validated, and that is
//! the whole of user isolation: the function serves every user and its IAM
//! policy covers every partition, so a handler taking an owner from the client
//! would defeat it (DR-0010, DR-0015).

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use shared::{ActionType, NewActionType, icon_names};

use crate::identity::Owner;
use crate::store::{Store, StoreError};

/// Long enough for any label worth putting on a record, short enough that a
/// stored name cannot be a document. No design document fixes these; they are
/// chosen here so that something does.
const MAX_NAME: usize = 64;
const MAX_UNIT: usize = 16;

pub async fn list(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
) -> Result<Json<Vec<ActionType>>, Failure> {
    Ok(Json(store.list_action_types(&owner).await?))
}

pub async fn create(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Json(new): Json<NewActionType>,
) -> Result<(StatusCode, Json<ActionType>), Failure> {
    let new = validate(new)?;
    let created = store.create_action_type(&owner, new).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_one(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Path(id): Path<String>,
) -> Result<Json<ActionType>, Failure> {
    store
        .get_action_type(&owner, &id)
        .await?
        .map(Json)
        .ok_or_else(Failure::not_found)
}

pub async fn update(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Path(id): Path<String>,
    Json(new): Json<NewActionType>,
) -> Result<Json<ActionType>, Failure> {
    let new = validate(new)?;

    store
        .update_action_type(&owner, &id, new)
        .await?
        .map(Json)
        .ok_or_else(Failure::not_found)
}

pub async fn delete(
    State(store): State<Arc<Store>>,
    Owner(owner): Owner,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    store.delete_action_type(&owner, &id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// What may be stored.
///
/// The picker is the only control surface that offers an icon, but a request is
/// not obliged to have come from it, so the name is checked against the same
/// catalog the picker was built from (DR-0014). The two text fields are trimmed
/// rather than rejected for surrounding space: a trailing space is a typo, not
/// an intent.
fn validate(new: NewActionType) -> Result<NewActionType, Failure> {
    let name = new.name.trim().to_owned();
    let unit = new.unit.trim().to_owned();

    if name.is_empty() {
        return Err(Failure::rejected("An action name is required."));
    }
    if name.chars().count() > MAX_NAME {
        return Err(Failure::rejected(&format!(
            "An action name can be at most {MAX_NAME} characters."
        )));
    }
    if unit.is_empty() {
        return Err(Failure::rejected("A numeric unit is required."));
    }
    if unit.chars().count() > MAX_UNIT {
        return Err(Failure::rejected(&format!(
            "A numeric unit can be at most {MAX_UNIT} characters."
        )));
    }
    if !icon_names::is_known(&new.icon) {
        return Err(Failure::rejected(
            "That icon is not one of the supported icons.",
        ));
    }

    Ok(NewActionType {
        name,
        unit,
        icon: new.icon,
    })
}

/// Why a request produced no value.
///
/// The body is the reason in plain words, because that is what the screen
/// shows: a form reporting `400` where it could report `A numeric unit is
/// required.` is reporting the wrong thing.
#[derive(Debug)]
pub enum Failure {
    /// The request was understood and refused. The visitor can fix it.
    Rejected(String),
    /// The id named nothing in the caller's own partition — including when it
    /// names something in someone else's.
    NotFound(String),
    /// The store did not answer. The visitor cannot.
    Unavailable(String),
}

impl Failure {
    fn rejected(reason: &str) -> Self {
        Self::Rejected(reason.to_owned())
    }

    fn not_found() -> Self {
        Self::NotFound("That action type could not be found.".to_owned())
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

    fn proposed(name: &str, unit: &str, icon: &str) -> NewActionType {
        NewActionType {
            name: name.to_owned(),
            unit: unit.to_owned(),
            icon: icon.to_owned(),
        }
    }

    fn rejection(new: NewActionType) -> String {
        match validate(new) {
            Err(Failure::Rejected(reason)) => reason,
            Err(Failure::NotFound(reason)) => panic!("not found: {reason}"),
            Err(Failure::Unavailable(reason)) => panic!("unavailable: {reason}"),
            Ok(_) => panic!("accepted"),
        }
    }

    #[test]
    fn trims_rather_than_refuses_surrounding_space() {
        let accepted = validate(proposed("  Running ", " km ", "footprints")).unwrap();

        assert_eq!(accepted.name, "Running");
        assert_eq!(accepted.unit, "km");
    }

    /// The browser's `required` never sees this one: the field is not empty.
    #[test]
    fn refuses_a_name_that_is_only_space() {
        assert_eq!(
            rejection(proposed("   ", "km", "footprints")),
            "An action name is required."
        );
    }

    #[test]
    fn refuses_an_over_long_unit() {
        let rejected = rejection(proposed("Running", &"k".repeat(MAX_UNIT + 1), "footprints"));

        assert!(rejected.contains(&MAX_UNIT.to_string()), "{rejected}");
    }

    /// Counted in characters, not bytes, so a name of multi-byte characters is
    /// measured the way the person typing it would measure it.
    #[test]
    fn measures_length_in_characters() {
        assert!(validate(proposed(&"é".repeat(MAX_NAME), "km", "footprints")).is_ok());
    }

    #[test]
    fn refuses_an_icon_outside_the_catalog() {
        assert_eq!(
            rejection(proposed("Running", "km", "not-a-lucide-icon")),
            "That icon is not one of the supported icons."
        );
    }
}
