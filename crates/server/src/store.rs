//! Where action types are kept.
//!
//! One table, one partition per owner, and the key encoding
//! `docs/design/persistence.md` defines. Nothing here decides what may be
//! stored — [`crate::action_types`] validates, this writes.
//!
//! Two implementations, because development has no table. An enum rather than a
//! trait object: there are exactly two, they are chosen once at startup, and
//! keeping them concrete avoids making every method dyn-safe for a choice that
//! is never in question at runtime.

use std::collections::HashMap;
use std::sync::Mutex;

use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::AttributeValue;
use shared::{ActionType, NewActionType};
use time::OffsetDateTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use ulid::Ulid;

/// The environment variable `infra/api/lambda.tf` passes the table name in.
///
/// Its absence is what selects the in-memory store, so a deployed function is
/// the store it is because Terraform sets this, and `just dev-api` is the other
/// one because nothing does.
const TABLE_NAME: &str = "TABLE_NAME";

const PARTITION_PREFIX: &str = "USER#";
const TYPE_PREFIX: &str = "TYPE#";

/// Fixed-width RFC 3339 in UTC: always a `Z` offset, always three fractional
/// digits.
///
/// The sort key orders lexically, so a variable-width instant orders wrongly and
/// does it silently — the dashboard would show ten records, just not the ten
/// newest. Nothing else in the system enforces this format (DR-0015).
const TIMESTAMP: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

/// Anything that stopped a read or a write from happening.
///
/// One variant, because the caller does the same thing with all of them: the
/// request failed for a reason the visitor did not cause and cannot act on.
#[derive(Debug)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub enum Store {
    Dynamo {
        client: Box<Client>,
        table: String,
    },
    /// Development. Keyed by owner, and each owner's types are held in the
    /// order they were created, which is the order the table would return them.
    Memory(Mutex<HashMap<String, Vec<ActionType>>>),
}

impl Store {
    /// Whichever store the environment describes.
    pub async fn from_environment() -> Self {
        match std::env::var(TABLE_NAME) {
            Ok(table) if !table.is_empty() => {
                let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
                Self::Dynamo {
                    client: Box::new(Client::new(&config)),
                    table,
                }
            }
            _ => Self::Memory(Mutex::new(HashMap::new())),
        }
    }

    /// What this store is, for the line printed at startup. Which one is in use
    /// is the single most useful thing to know about a running instance.
    pub fn describe(&self) -> String {
        match self {
            Self::Dynamo { table, .. } => format!("DynamoDB table {table}"),
            Self::Memory(_) => format!("memory (no {TABLE_NAME} is set)"),
        }
    }

    /// One owner's action types, oldest first.
    ///
    /// `begins_with` on the sort key is what separates them from that owner's
    /// records, and the ULID in the key is what makes key order creation order.
    pub async fn list_action_types(&self, owner: &str) -> Result<Vec<ActionType>, StoreError> {
        match self {
            Self::Dynamo { client, table } => {
                let response = client
                    .query()
                    .table_name(table)
                    .key_condition_expression("pk = :pk AND begins_with(sk, :prefix)")
                    .expression_attribute_values(":pk", AttributeValue::S(partition(owner)))
                    .expression_attribute_values(
                        ":prefix",
                        AttributeValue::S(TYPE_PREFIX.to_owned()),
                    )
                    .send()
                    .await
                    .map_err(|err| StoreError(format!("could not read action types: {err}")))?;

                Ok(response
                    .items
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(action_type)
                    .collect())
            }
            Self::Memory(types) => Ok(types
                .lock()
                .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                .get(owner)
                .cloned()
                .unwrap_or_default()),
        }
    }

    /// One action type by id, or `None` if it is not in this owner's
    /// partition — including when it belongs to someone else's.
    pub async fn get_action_type(
        &self,
        owner: &str,
        id: &str,
    ) -> Result<Option<ActionType>, StoreError> {
        match self {
            Self::Dynamo { client, table } => {
                let response = client
                    .get_item()
                    .table_name(table)
                    .key("pk", AttributeValue::S(partition(owner)))
                    .key("sk", AttributeValue::S(format!("{TYPE_PREFIX}{id}")))
                    .send()
                    .await
                    .map_err(|err| StoreError(format!("could not read the action type: {err}")))?;

                Ok(response.item.and_then(action_type))
            }
            Self::Memory(types) => Ok(types
                .lock()
                .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                .get(owner)
                .and_then(|owned| owned.iter().find(|existing| existing.id == id).cloned())),
        }
    }

    /// Registers one, and answers with it as stored.
    ///
    /// The identifier is minted here rather than accepted from the client, and
    /// what the API exposes is the bare ULID: a client holding `TYPE#01J…`
    /// would be holding a storage detail this could not then change.
    pub async fn create_action_type(
        &self,
        owner: &str,
        new: NewActionType,
    ) -> Result<ActionType, StoreError> {
        let id = Ulid::generate().to_string();
        let created_at = now()?;

        let action_type = ActionType {
            id,
            name: new.name,
            unit: new.unit,
            icon: new.icon,
        };

        match self {
            Self::Dynamo { client, table } => {
                client
                    .put_item()
                    .table_name(table)
                    .item("pk", AttributeValue::S(partition(owner)))
                    .item(
                        "sk",
                        AttributeValue::S(format!("{TYPE_PREFIX}{}", action_type.id)),
                    )
                    .item("name", AttributeValue::S(action_type.name.clone()))
                    .item("unit", AttributeValue::S(action_type.unit.clone()))
                    .item("icon", AttributeValue::S(action_type.icon.clone()))
                    .item("created_at", AttributeValue::S(created_at))
                    .send()
                    .await
                    .map_err(|err| StoreError(format!("could not store the action type: {err}")))?;
            }
            Self::Memory(types) => {
                types
                    .lock()
                    .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                    .entry(owner.to_owned())
                    .or_default()
                    .push(action_type.clone());
            }
        }

        Ok(action_type)
    }

    /// Changes what is stored for one action type, and answers with it as
    /// stored — or `None` if `id` is not in this owner's partition, which this
    /// refuses to treat as permission to create it there.
    ///
    /// The identifier and creation time are untouched; only the three fields a
    /// client may propose change. Existing records that copied this type's
    /// prior name, unit or icon are untouched too — DR-0016 is what makes that
    /// the correct behaviour rather than a gap.
    pub async fn update_action_type(
        &self,
        owner: &str,
        id: &str,
        new: NewActionType,
    ) -> Result<Option<ActionType>, StoreError> {
        let action_type = ActionType {
            id: id.to_owned(),
            name: new.name,
            unit: new.unit,
            icon: new.icon,
        };

        match self {
            Self::Dynamo { client, table } => {
                let result = client
                    .update_item()
                    .table_name(table)
                    .key("pk", AttributeValue::S(partition(owner)))
                    .key("sk", AttributeValue::S(format!("{TYPE_PREFIX}{id}")))
                    // Refuses to bring an item into existence: an edit changes
                    // what is there, and what is there is what this checks for.
                    .condition_expression("attribute_exists(pk)")
                    .update_expression("SET #name = :name, #unit = :unit, #icon = :icon")
                    // All three aliased rather than just the one DynamoDB is
                    // known to reserve (`NAME`): cheaper than checking whether
                    // `UNIT` or `ICON` are reserved words too.
                    .expression_attribute_names("#name", "name")
                    .expression_attribute_names("#unit", "unit")
                    .expression_attribute_names("#icon", "icon")
                    .expression_attribute_values(
                        ":name",
                        AttributeValue::S(action_type.name.clone()),
                    )
                    .expression_attribute_values(
                        ":unit",
                        AttributeValue::S(action_type.unit.clone()),
                    )
                    .expression_attribute_values(
                        ":icon",
                        AttributeValue::S(action_type.icon.clone()),
                    )
                    .send()
                    .await;

                match result {
                    Ok(_) => Ok(Some(action_type)),
                    Err(err) => match err.as_service_error() {
                        Some(service_err)
                            if service_err.is_conditional_check_failed_exception() =>
                        {
                            Ok(None)
                        }
                        _ => Err(StoreError(format!(
                            "could not update the action type: {err}"
                        ))),
                    },
                }
            }
            Self::Memory(types) => {
                let mut types = types
                    .lock()
                    .map_err(|_| StoreError("the store is poisoned".to_owned()))?;
                let owned = types.entry(owner.to_owned()).or_default();

                // In place, so an edit keeps the position creation order gave
                // it — the same order the `sk` would keep it in, since only
                // non-key attributes change.
                match owned.iter_mut().find(|existing| existing.id == id) {
                    Some(existing) => {
                        *existing = action_type.clone();
                        Ok(Some(action_type))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    /// Removes one action type. Idempotent: whether `id` was there or not, the
    /// answer is the same, because `DeleteItem` already behaves this way and
    /// there is no design requirement to tell the two cases apart.
    ///
    /// This never touches a `RECORD#` item. DR-0016 is why that is correct
    /// rather than incomplete: a record carries its own copy of what it needs
    /// to display, so deleting the type it points to cannot orphan anything a
    /// screen would try to read from the type itself.
    pub async fn delete_action_type(&self, owner: &str, id: &str) -> Result<(), StoreError> {
        match self {
            Self::Dynamo { client, table } => {
                client
                    .delete_item()
                    .table_name(table)
                    .key("pk", AttributeValue::S(partition(owner)))
                    .key("sk", AttributeValue::S(format!("{TYPE_PREFIX}{id}")))
                    .send()
                    .await
                    .map_err(|err| {
                        StoreError(format!("could not delete the action type: {err}"))
                    })?;
            }
            Self::Memory(types) => {
                if let Some(owned) = types
                    .lock()
                    .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                    .get_mut(owner)
                {
                    owned.retain(|existing| existing.id != id);
                }
            }
        }

        Ok(())
    }
}

fn partition(owner: &str) -> String {
    format!("{PARTITION_PREFIX}{owner}")
}

/// One stored item as the wire sees it, or nothing if it is not one.
///
/// A missing attribute drops the item rather than failing the request: the
/// alternative is one malformed row making a whole screen unreachable.
fn action_type(item: HashMap<String, AttributeValue>) -> Option<ActionType> {
    let id = string(&item, "sk")?.strip_prefix(TYPE_PREFIX)?.to_owned();

    Some(ActionType {
        id,
        name: string(&item, "name")?.to_owned(),
        unit: string(&item, "unit")?.to_owned(),
        icon: string(&item, "icon")?.to_owned(),
    })
}

fn string<'item>(item: &'item HashMap<String, AttributeValue>, key: &str) -> Option<&'item str> {
    item.get(key)?.as_s().ok().map(String::as_str)
}

fn now() -> Result<String, StoreError> {
    OffsetDateTime::now_utc()
        .format(TIMESTAMP)
        .map_err(|err| StoreError(format!("could not format the current time: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_fixed_width() {
        // `2026-08-10T13:24:05.007Z` — 24 characters, always.
        assert_eq!(now().unwrap().len(), 24);
    }

    #[test]
    fn an_item_without_its_prefix_is_not_an_action_type() {
        let item = HashMap::from([
            (
                "sk".to_owned(),
                AttributeValue::S("RECORD#2026-08-10T00:00:00.000Z#01J".to_owned()),
            ),
            ("name".to_owned(), AttributeValue::S("Running".to_owned())),
            ("unit".to_owned(), AttributeValue::S("km".to_owned())),
            (
                "icon".to_owned(),
                AttributeValue::S("footprints".to_owned()),
            ),
        ]);

        assert!(action_type(item).is_none());
    }

    #[test]
    fn an_action_type_exposes_the_bare_ulid() {
        let item = HashMap::from([
            (
                "sk".to_owned(),
                AttributeValue::S("TYPE#01JZZZZZZZZZZZZZZZZZZZZZZZ".to_owned()),
            ),
            ("name".to_owned(), AttributeValue::S("Running".to_owned())),
            ("unit".to_owned(), AttributeValue::S("km".to_owned())),
            (
                "icon".to_owned(),
                AttributeValue::S("footprints".to_owned()),
            ),
        ]);

        assert_eq!(action_type(item).unwrap().id, "01JZZZZZZZZZZZZZZZZZZZZZZZ");
    }

    #[tokio::test]
    async fn the_memory_store_keeps_creation_order_per_owner() {
        let store = Store::Memory(Mutex::new(HashMap::new()));
        for name in ["Running", "Water"] {
            store
                .create_action_type(
                    "someone",
                    NewActionType {
                        name: name.to_owned(),
                        unit: "km".to_owned(),
                        icon: "footprints".to_owned(),
                    },
                )
                .await
                .unwrap();
        }
        store
            .create_action_type(
                "someone-else",
                NewActionType {
                    name: "Reading".to_owned(),
                    unit: "pages".to_owned(),
                    icon: "book-open".to_owned(),
                },
            )
            .await
            .unwrap();

        let types = store.list_action_types("someone").await.unwrap();
        let names: Vec<&str> = types.iter().map(|one| one.name.as_str()).collect();

        assert_eq!(names, ["Running", "Water"]);
    }

    fn proposed(name: &str, unit: &str, icon: &str) -> NewActionType {
        NewActionType {
            name: name.to_owned(),
            unit: unit.to_owned(),
            icon: icon.to_owned(),
        }
    }

    #[tokio::test]
    async fn a_fetched_type_is_none_outside_its_owners_partition() {
        let store = Store::Memory(Mutex::new(HashMap::new()));
        let created = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();

        assert!(
            store
                .get_action_type("someone-else", &created.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store
                .get_action_type("someone", &created.id)
                .await
                .unwrap()
                .unwrap()
                .name,
            "Running"
        );
    }

    #[tokio::test]
    async fn updating_keeps_the_id_and_creation_order_but_changes_the_rest() {
        let store = Store::Memory(Mutex::new(HashMap::new()));
        let first = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();
        store
            .create_action_type("someone", proposed("Water", "glasses", "droplets"))
            .await
            .unwrap();

        let updated = store
            .update_action_type("someone", &first.id, proposed("Jogging", "mi", "timer"))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.id, first.id);
        assert_eq!(updated.name, "Jogging");

        let types = store.list_action_types("someone").await.unwrap();
        let names: Vec<&str> = types.iter().map(|one| one.name.as_str()).collect();
        assert_eq!(names, ["Jogging", "Water"]);
    }

    /// An id that belongs to someone else's partition is not this owner's to
    /// edit — answered as "not found" rather than silently creating one.
    #[tokio::test]
    async fn updating_a_stranger_to_the_partition_answers_none() {
        let store = Store::Memory(Mutex::new(HashMap::new()));

        let result = store
            .update_action_type(
                "someone",
                "not-a-real-id",
                proposed("Jogging", "mi", "timer"),
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn deleting_removes_only_the_named_type() {
        let store = Store::Memory(Mutex::new(HashMap::new()));
        let first = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();
        store
            .create_action_type("someone", proposed("Water", "glasses", "droplets"))
            .await
            .unwrap();

        store
            .delete_action_type("someone", &first.id)
            .await
            .unwrap();

        let types = store.list_action_types("someone").await.unwrap();
        let names: Vec<&str> = types.iter().map(|one| one.name.as_str()).collect();
        assert_eq!(names, ["Water"]);
    }

    /// `DeleteItem` does not fail on a missing key, and neither does this.
    #[tokio::test]
    async fn deleting_a_type_that_is_not_there_is_not_an_error() {
        let store = Store::Memory(Mutex::new(HashMap::new()));

        store
            .delete_action_type("someone", "not-a-real-id")
            .await
            .unwrap();
    }
}
