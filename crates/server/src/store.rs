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
use shared::{
    ActionRecord, ActionType, NewActionRecord, NewActionType, RecentSummary, UpdateActionRecord,
};
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{Duration, OffsetDateTime};
use ulid::Ulid;

/// The environment variable `infra/api/lambda.tf` passes the table name in.
///
/// Its absence is what selects the in-memory store, so a deployed function is
/// the store it is because Terraform sets this, and `just dev-api` is the other
/// one because nothing does.
const TABLE_NAME: &str = "TABLE_NAME";

const PARTITION_PREFIX: &str = "USER#";
const TYPE_PREFIX: &str = "TYPE#";
const RECORD_PREFIX: &str = "RECORD#";

/// Fixed-width RFC 3339 in UTC: always a `Z` offset, always three fractional
/// digits.
///
/// The sort key orders lexically, so a variable-width instant orders wrongly and
/// does it silently — the dashboard would show ten records, just not the ten
/// newest. Nothing else in the system enforces this format (DR-0015).
const TIMESTAMP: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

/// The dashboard's recent-actions cap — `page-layouts.md`. Independent of
/// [`SUMMARY_WINDOW_DAYS`]; the two need not agree on count, and
/// `persistence.md` says so explicitly.
const RECENT_RECORDS_LIMIT: i32 = 10;

/// The dashboard's summary window, in days — `page-layouts.md`,
/// `persistence.md`. [`RecentSummary::daily`] always has exactly this many
/// entries.
const SUMMARY_WINDOW_DAYS: i64 = 10;

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
    /// Development. Keyed by owner, one map per entity kind — mirroring the
    /// one DynamoDB table both live in, split by `sk` prefix instead of by
    /// map. Each owner's types and records are held in the order they were
    /// created, which is the order the table would return them.
    Memory {
        types: Mutex<HashMap<String, Vec<ActionType>>>,
        records: Mutex<HashMap<String, Vec<ActionRecord>>>,
    },
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
            _ => Self::Memory {
                types: Mutex::new(HashMap::new()),
                records: Mutex::new(HashMap::new()),
            },
        }
    }

    /// What this store is, for the line printed at startup. Which one is in use
    /// is the single most useful thing to know about a running instance.
    pub fn describe(&self) -> String {
        match self {
            Self::Dynamo { table, .. } => format!("DynamoDB table {table}"),
            Self::Memory { .. } => format!("memory (no {TABLE_NAME} is set)"),
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
            Self::Memory { types, .. } => Ok(types
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
            Self::Memory { types, .. } => Ok(types
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
            Self::Memory { types, .. } => {
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
            Self::Memory { types, .. } => {
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
            Self::Memory { types, .. } => {
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

    /// One owner's action records, newest first — the reverse of
    /// [`Store::list_action_types`], because a history reads newest first
    /// where a set of registered types reads in the order they were added.
    /// No cap: the actions screen shows the account's full history
    /// (`page-layouts.md`), unlike [`Store::recent_action_records`].
    pub async fn list_action_records(&self, owner: &str) -> Result<Vec<ActionRecord>, StoreError> {
        self.query_records(owner, None).await
    }

    /// The dashboard's recent-actions list: the newest
    /// [`RECENT_RECORDS_LIMIT`] records — a separate limit from
    /// [`Store::recent_summary`]'s window, not a smaller version of the same
    /// one (`persistence.md`).
    pub async fn recent_action_records(
        &self,
        owner: &str,
    ) -> Result<Vec<ActionRecord>, StoreError> {
        self.query_records(owner, Some(RECENT_RECORDS_LIMIT)).await
    }

    /// The `begins_with(sk, "RECORD#")` query both [`Store::list_action_records`]
    /// and [`Store::recent_action_records`] run, newest first, differing only
    /// in whether the result is capped.
    async fn query_records(
        &self,
        owner: &str,
        limit: Option<i32>,
    ) -> Result<Vec<ActionRecord>, StoreError> {
        match self {
            Self::Dynamo { client, table } => {
                let mut request = client
                    .query()
                    .table_name(table)
                    .key_condition_expression("pk = :pk AND begins_with(sk, :prefix)")
                    .expression_attribute_values(":pk", AttributeValue::S(partition(owner)))
                    .expression_attribute_values(
                        ":prefix",
                        AttributeValue::S(RECORD_PREFIX.to_owned()),
                    )
                    .scan_index_forward(false);
                if let Some(limit) = limit {
                    request = request.limit(limit);
                }

                let response = request
                    .send()
                    .await
                    .map_err(|err| StoreError(format!("could not read actions: {err}")))?;

                Ok(response
                    .items
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(action_record)
                    .collect())
            }
            Self::Memory { records, .. } => {
                let newest_first: Vec<ActionRecord> = records
                    .lock()
                    .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                    .get(owner)
                    .map(|owned| owned.iter().rev().cloned().collect())
                    .unwrap_or_default();

                Ok(match limit {
                    Some(limit) => newest_first.into_iter().take(limit as usize).collect(),
                    None => newest_first,
                })
            }
        }
    }

    /// The dashboard's ten-day summary: one count per UTC calendar day,
    /// oldest first, covering today and the nine days before it — a separate
    /// query and a separate limit from [`Store::recent_action_records`]'s
    /// list (`persistence.md`). `recorded_at` is always UTC and no per-user
    /// timezone is captured anywhere in this design, so "today" here is the
    /// UTC date, not the visitor's local one.
    ///
    /// DynamoDB does no aggregation, so bucketing the matched records by day
    /// is this method's own job, done by comparing each one's `recorded_at`
    /// against the day boundaries as fixed-width strings — the same
    /// lexical-order property the rest of this file already relies on,
    /// rather than parsing a timestamp back into a date.
    pub async fn recent_summary(&self, owner: &str) -> Result<RecentSummary, StoreError> {
        let today = OffsetDateTime::now_utc().date();
        let window_start = today - Duration::days(SUMMARY_WINDOW_DAYS - 1);

        // One bound per day in the window, plus the exclusive upper bound:
        // `SUMMARY_WINDOW_DAYS + 1` fenceposts for `SUMMARY_WINDOW_DAYS`
        // buckets.
        let mut bounds = Vec::with_capacity(SUMMARY_WINDOW_DAYS as usize + 1);
        for offset in 0..=SUMMARY_WINDOW_DAYS {
            bounds.push(format_instant(
                (window_start + Duration::days(offset))
                    .midnight()
                    .assume_utc(),
            )?);
        }
        let from = &bounds[0];
        let to = &bounds[SUMMARY_WINDOW_DAYS as usize];

        let items: Vec<ActionRecord> = match self {
            Self::Dynamo { client, table } => {
                let response = client
                    .query()
                    .table_name(table)
                    .key_condition_expression("pk = :pk AND sk BETWEEN :from AND :to")
                    .expression_attribute_values(":pk", AttributeValue::S(partition(owner)))
                    .expression_attribute_values(
                        ":from",
                        AttributeValue::S(format!("{RECORD_PREFIX}{from}")),
                    )
                    // `BETWEEN` is inclusive at both ends, but every stored
                    // key has a `#<ulid>` suffix and so sorts strictly after
                    // the bare `RECORD#<to>` bound — the window's upper end
                    // is exclusive with no off-by-one-millisecond adjustment
                    // (`persistence.md`).
                    .expression_attribute_values(
                        ":to",
                        AttributeValue::S(format!("{RECORD_PREFIX}{to}")),
                    )
                    .send()
                    .await
                    .map_err(|err| {
                        StoreError(format!("could not read the summary window: {err}"))
                    })?;

                response
                    .items
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(action_record)
                    .collect()
            }
            Self::Memory { records, .. } => records
                .lock()
                .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                .get(owner)
                .map(|owned| {
                    owned
                        .iter()
                        .filter(|record| &record.recorded_at >= from && &record.recorded_at < to)
                        .cloned()
                        .collect()
                })
                .unwrap_or_default(),
        };

        let mut daily = vec![0u32; SUMMARY_WINDOW_DAYS as usize];
        for record in &items {
            if let Some(day) = bounds
                .windows(2)
                .position(|pair| record.recorded_at >= pair[0] && record.recorded_at < pair[1])
            {
                daily[day] += 1;
            }
        }

        Ok(RecentSummary {
            total: daily.iter().sum(),
            daily,
        })
    }

    /// Records one action against a registered type, copying that type's
    /// display attributes as they stand right now (DR-0016). `None` means
    /// `type_id` names no action type in this owner's own partition — not a
    /// [`StoreError`], because the caller sent a name that does not resolve
    /// rather than the store failing to answer.
    pub async fn create_action_record(
        &self,
        owner: &str,
        new: NewActionRecord,
    ) -> Result<Option<ActionRecord>, StoreError> {
        let Some(action_type) = self.get_action_type(owner, &new.type_id).await? else {
            return Ok(None);
        };

        let id = Ulid::generate().to_string();
        let recorded_at = now()?;
        let record = ActionRecord {
            id,
            action_type,
            value: new.value,
            recorded_at,
        };

        match self {
            Self::Dynamo { client, table } => {
                client
                    .put_item()
                    .table_name(table)
                    .item("pk", AttributeValue::S(partition(owner)))
                    .item(
                        "sk",
                        AttributeValue::S(record_key(&record.recorded_at, &record.id)),
                    )
                    .item("type_id", AttributeValue::S(record.action_type.id.clone()))
                    .item("name", AttributeValue::S(record.action_type.name.clone()))
                    .item("unit", AttributeValue::S(record.action_type.unit.clone()))
                    .item("icon", AttributeValue::S(record.action_type.icon.clone()))
                    .item("value", AttributeValue::N(record.value.to_string()))
                    .item("recorded_at", AttributeValue::S(record.recorded_at.clone()))
                    .send()
                    .await
                    .map_err(|err| StoreError(format!("could not store the action: {err}")))?;
            }
            Self::Memory { records, .. } => {
                records
                    .lock()
                    .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                    .entry(owner.to_owned())
                    .or_default()
                    .push(record.clone());
            }
        }

        Ok(Some(record))
    }

    /// One action record by id, or `None` if it is not in this owner's
    /// partition.
    ///
    /// Unlike an action type's `TYPE#<ulid>`, a record's key is
    /// `RECORD#<recorded_at>#<ulid>` — the id alone cannot reconstruct it, so
    /// the `Dynamo` variant queries the owner's whole `RECORD#` range and
    /// matches the trailing id instead of adding a secondary index for a
    /// pattern this project's scale does not yet need (see the Work Log this
    /// choice was confirmed in, and the Decision Record it produced).
    pub async fn get_action_record(
        &self,
        owner: &str,
        id: &str,
    ) -> Result<Option<ActionRecord>, StoreError> {
        match self {
            Self::Dynamo { client, table } => find_action_record(client, table, owner, id).await,
            Self::Memory { records, .. } => Ok(records
                .lock()
                .map_err(|_| StoreError("the store is poisoned".to_owned()))?
                .get(owner)
                .and_then(|owned| owned.iter().find(|existing| existing.id == id).cloned())),
        }
    }

    /// Changes the recorded value, and answers with the record as stored — or
    /// `None` if `id` is not in this owner's partition. Everything else about
    /// the record — its type, the copied display attributes, `recorded_at` —
    /// is untouched; only `value` is ever editable (DR-0016).
    pub async fn update_action_record(
        &self,
        owner: &str,
        id: &str,
        new: UpdateActionRecord,
    ) -> Result<Option<ActionRecord>, StoreError> {
        match self {
            Self::Dynamo { client, table } => {
                let Some(mut record) = find_action_record(client, table, owner, id).await? else {
                    return Ok(None);
                };

                match client
                    .update_item()
                    .table_name(table)
                    .key("pk", AttributeValue::S(partition(owner)))
                    .key(
                        "sk",
                        AttributeValue::S(record_key(&record.recorded_at, &record.id)),
                    )
                    // Guards against a delete landing between the query above
                    // and this write, the same defence `update_action_type`
                    // uses against creating an item that should not exist.
                    .condition_expression("attribute_exists(pk)")
                    .update_expression("SET #value = :value")
                    .expression_attribute_names("#value", "value")
                    .expression_attribute_values(":value", AttributeValue::N(new.value.to_string()))
                    .send()
                    .await
                {
                    Ok(_) => {
                        record.value = new.value;
                        Ok(Some(record))
                    }
                    Err(err) => match err.as_service_error() {
                        Some(service_err)
                            if service_err.is_conditional_check_failed_exception() =>
                        {
                            Ok(None)
                        }
                        _ => Err(StoreError(format!("could not update the action: {err}"))),
                    },
                }
            }
            Self::Memory { records, .. } => {
                let mut records = records
                    .lock()
                    .map_err(|_| StoreError("the store is poisoned".to_owned()))?;
                let owned = records.entry(owner.to_owned()).or_default();

                match owned.iter_mut().find(|existing| existing.id == id) {
                    Some(existing) => {
                        existing.value = new.value;
                        Ok(Some(existing.clone()))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    /// Removes one action record. Idempotent, like [`Store::delete_action_type`]:
    /// whether `id` was there or not, the answer is the same.
    pub async fn delete_action_record(&self, owner: &str, id: &str) -> Result<(), StoreError> {
        match self {
            Self::Dynamo { client, table } => {
                if let Some(record) = find_action_record(client, table, owner, id).await? {
                    client
                        .delete_item()
                        .table_name(table)
                        .key("pk", AttributeValue::S(partition(owner)))
                        .key(
                            "sk",
                            AttributeValue::S(record_key(&record.recorded_at, &record.id)),
                        )
                        .send()
                        .await
                        .map_err(|err| StoreError(format!("could not delete the action: {err}")))?;
                }
            }
            Self::Memory { records, .. } => {
                if let Some(owned) = records
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

/// The full sort key for one action record, given the two components its
/// bare id does not carry.
fn record_key(recorded_at: &str, id: &str) -> String {
    format!("{RECORD_PREFIX}{recorded_at}#{id}")
}

/// One stored item as the wire sees it, or nothing if it is not a record —
/// including a `TYPE#` item, which `strip_prefix` rejects the same way
/// [`action_type`] rejects a `RECORD#` one.
fn action_record(item: HashMap<String, AttributeValue>) -> Option<ActionRecord> {
    let id = string(&item, "sk")?
        .strip_prefix(RECORD_PREFIX)?
        .rsplit_once('#')?
        .1
        .to_owned();
    let value: f64 = item.get("value")?.as_n().ok()?.parse().ok()?;

    Some(ActionRecord {
        id,
        action_type: ActionType {
            id: string(&item, "type_id")?.to_owned(),
            name: string(&item, "name")?.to_owned(),
            unit: string(&item, "unit")?.to_owned(),
            icon: string(&item, "icon")?.to_owned(),
        },
        value,
        recorded_at: string(&item, "recorded_at")?.to_owned(),
    })
}

/// Locates one action record by the bare id the API exposes.
///
/// A record's key is `RECORD#<recorded_at>#<ulid>` — unlike an action type's
/// `TYPE#<ulid>`, the id alone cannot reconstruct it, so this queries the
/// owner's whole `RECORD#` range and matches the trailing id, rather than
/// adding a secondary index for a pattern this project's scale does not yet
/// need.
async fn find_action_record(
    client: &Client,
    table: &str,
    owner: &str,
    id: &str,
) -> Result<Option<ActionRecord>, StoreError> {
    let response = client
        .query()
        .table_name(table)
        .key_condition_expression("pk = :pk AND begins_with(sk, :prefix)")
        .expression_attribute_values(":pk", AttributeValue::S(partition(owner)))
        .expression_attribute_values(":prefix", AttributeValue::S(RECORD_PREFIX.to_owned()))
        .send()
        .await
        .map_err(|err| StoreError(format!("could not read actions: {err}")))?;

    Ok(response
        .items
        .unwrap_or_default()
        .into_iter()
        .filter_map(action_record)
        .find(|record| record.id == id))
}

fn string<'item>(item: &'item HashMap<String, AttributeValue>, key: &str) -> Option<&'item str> {
    item.get(key)?.as_s().ok().map(String::as_str)
}

/// Formats one instant as this store's fixed-width `TIMESTAMP` — shared by
/// [`now`] and [`Store::recent_summary`]'s day boundaries, so both go through
/// the one format the sort key's lexical order depends on.
fn format_instant(instant: OffsetDateTime) -> Result<String, StoreError> {
    instant
        .format(TIMESTAMP)
        .map_err(|err| StoreError(format!("could not format a timestamp: {err}")))
}

fn now() -> Result<String, StoreError> {
    format_instant(OffsetDateTime::now_utc())
}

#[cfg(test)]
impl Store {
    /// An empty `Memory` store, for tests that do not care what is in it —
    /// only this file's tests and `main.rs`'s router-level tests construct
    /// one, and both would otherwise repeat the two-map literal by hand.
    pub(crate) fn memory() -> Self {
        Self::Memory {
            types: Mutex::new(HashMap::new()),
            records: Mutex::new(HashMap::new()),
        }
    }
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
        let store = Store::memory();
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
        let store = Store::memory();
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
        let store = Store::memory();
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
        let store = Store::memory();

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
        let store = Store::memory();
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
        let store = Store::memory();

        store
            .delete_action_type("someone", "not-a-real-id")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn creating_a_record_copies_the_types_current_display_attributes() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();

        let created = store
            .create_action_record(
                "someone",
                NewActionRecord {
                    type_id: action_type.id.clone(),
                    value: 5.2,
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(created.action_type, action_type);
        assert_eq!(created.value, 5.2);
        assert_eq!(created.recorded_at.len(), 24);
    }

    /// A record does not follow the type it was created from: DR-0016.
    #[tokio::test]
    async fn editing_a_type_does_not_change_a_records_copied_attributes() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();
        let created = store
            .create_action_record(
                "someone",
                NewActionRecord {
                    type_id: action_type.id.clone(),
                    value: 5.2,
                },
            )
            .await
            .unwrap()
            .unwrap();

        store
            .update_action_type(
                "someone",
                &action_type.id,
                proposed("Jogging", "mi", "timer"),
            )
            .await
            .unwrap();

        let unchanged = store
            .get_action_record("someone", &created.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.action_type.name, "Running");
    }

    /// `type_id` naming nothing in this owner's partition is `None`, the same
    /// shape `update_action_type` already answers a stranger id with.
    #[tokio::test]
    async fn creating_a_record_for_an_unknown_type_answers_none() {
        let store = Store::memory();

        let created = store
            .create_action_record(
                "someone",
                NewActionRecord {
                    type_id: "not-a-real-id".to_owned(),
                    value: 1.0,
                },
            )
            .await
            .unwrap();

        assert!(created.is_none());
    }

    #[tokio::test]
    async fn listing_records_is_newest_first() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();

        let mut created_values = Vec::new();
        for value in [1.0, 2.0, 3.0] {
            let record = store
                .create_action_record(
                    "someone",
                    NewActionRecord {
                        type_id: action_type.id.clone(),
                        value,
                    },
                )
                .await
                .unwrap()
                .unwrap();
            created_values.push(record.value);
        }

        let listed = store.list_action_records("someone").await.unwrap();
        let values: Vec<f64> = listed.iter().map(|record| record.value).collect();
        assert_eq!(values, [3.0, 2.0, 1.0]);
    }

    #[tokio::test]
    async fn a_fetched_record_is_none_outside_its_owners_partition() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();
        let created = store
            .create_action_record(
                "someone",
                NewActionRecord {
                    type_id: action_type.id,
                    value: 5.2,
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert!(
            store
                .get_action_record("someone-else", &created.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn updating_a_record_changes_only_the_value() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();
        let created = store
            .create_action_record(
                "someone",
                NewActionRecord {
                    type_id: action_type.id,
                    value: 5.2,
                },
            )
            .await
            .unwrap()
            .unwrap();

        let updated = store
            .update_action_record("someone", &created.id, UpdateActionRecord { value: 6.0 })
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.value, 6.0);
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.recorded_at, created.recorded_at);
        assert_eq!(updated.action_type, created.action_type);
    }

    #[tokio::test]
    async fn updating_a_stranger_record_answers_none() {
        let store = Store::memory();

        let result = store
            .update_action_record(
                "someone",
                "not-a-real-id",
                UpdateActionRecord { value: 1.0 },
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn deleting_removes_only_the_named_record() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();
        let first = store
            .create_action_record(
                "someone",
                NewActionRecord {
                    type_id: action_type.id.clone(),
                    value: 1.0,
                },
            )
            .await
            .unwrap()
            .unwrap();
        store
            .create_action_record(
                "someone",
                NewActionRecord {
                    type_id: action_type.id,
                    value: 2.0,
                },
            )
            .await
            .unwrap();

        store
            .delete_action_record("someone", &first.id)
            .await
            .unwrap();

        let remaining = store.list_action_records("someone").await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].value, 2.0);
    }

    /// `DeleteItem` does not fail on a missing key, and neither does this.
    #[tokio::test]
    async fn deleting_a_record_that_is_not_there_is_not_an_error() {
        let store = Store::memory();

        store
            .delete_action_record("someone", "not-a-real-id")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn recent_action_records_caps_at_ten_newest_first() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();

        for value in 0..12 {
            store
                .create_action_record(
                    "someone",
                    NewActionRecord {
                        type_id: action_type.id.clone(),
                        value: f64::from(value),
                    },
                )
                .await
                .unwrap();
        }

        let recent = store.recent_action_records("someone").await.unwrap();
        let values: Vec<f64> = recent.iter().map(|record| record.value).collect();
        assert_eq!(values, [11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0]);
    }

    #[tokio::test]
    async fn recent_summary_of_an_empty_partition_is_ten_zero_buckets() {
        let store = Store::memory();

        let summary = store.recent_summary("someone").await.unwrap();

        assert_eq!(summary.total, 0);
        assert_eq!(summary.daily, vec![0; 10]);
    }

    /// Records created "now" always land in today's bucket — the last one,
    /// since `daily` is oldest first. This is what a test can assert without
    /// controlling the clock `recent_summary` reads.
    #[tokio::test]
    async fn recent_summary_counts_todays_records_into_the_last_bucket() {
        let store = Store::memory();
        let action_type = store
            .create_action_type("someone", proposed("Running", "km", "footprints"))
            .await
            .unwrap();

        for _ in 0..3 {
            store
                .create_action_record(
                    "someone",
                    NewActionRecord {
                        type_id: action_type.id.clone(),
                        value: 1.0,
                    },
                )
                .await
                .unwrap();
        }

        let summary = store.recent_summary("someone").await.unwrap();

        assert_eq!(summary.total, 3);
        assert_eq!(summary.daily.len(), 10);
        assert_eq!(summary.daily[9], 3);
        assert_eq!(summary.daily[..9], [0; 9]);
    }

    /// A stranger's records neither inflate the total nor leak into any
    /// bucket — the same partition isolation every other query in this file
    /// already holds to.
    #[tokio::test]
    async fn recent_summary_does_not_count_another_owners_records() {
        let store = Store::memory();
        let stranger_type = store
            .create_action_type("someone-else", proposed("Running", "km", "footprints"))
            .await
            .unwrap();
        store
            .create_action_record(
                "someone-else",
                NewActionRecord {
                    type_id: stranger_type.id,
                    value: 1.0,
                },
            )
            .await
            .unwrap();

        let summary = store.recent_summary("someone").await.unwrap();

        assert_eq!(summary.total, 0);
    }
}

/// The `Dynamo` variant, run against DynamoDB Local instead of `Memory` —
/// `just test-dynamo` starts it, creates the table, and runs these. Ignored by
/// default so `just test`/`cargo test --workspace` stays independent of Java
/// and a running table (DR-0020, testing.md).
///
/// These repeat a subset of the `Memory` tests above rather than covering new
/// behaviour: the point is that the same assertions hold against the real
/// `Query`/`GetItem`/`UpdateItem`/`DeleteItem` encoding, not the `Vec`/`HashMap`
/// standing in for it. Each test uses a freshly minted owner so repeated runs
/// against a table that outlives one `cargo test` invocation cannot collide —
/// `just dynamo`'s `-inMemory` normally makes that moot, but nothing here
/// depends on that.
#[cfg(test)]
mod dynamo_tests {
    use super::*;

    /// Panics with a pointer to `just test-dynamo` rather than silently
    /// falling back to `Memory`, which `Store::from_environment` would do if
    /// `TABLE_NAME` were unset — the one outcome that would make every test
    /// below pass without checking anything.
    async fn dynamo_store() -> Store {
        assert!(
            std::env::var(TABLE_NAME).is_ok(),
            "{TABLE_NAME} is not set — run `just test-dynamo`, which starts \
             DynamoDB Local, creates the table, and sets the environment \
             these tests need before running them"
        );
        let store = Store::from_environment().await;
        assert!(
            matches!(store, Store::Dynamo { .. }),
            "TABLE_NAME is set but Store::from_environment chose Memory anyway"
        );
        store
    }

    fn unique_owner(label: &str) -> String {
        format!("test-{label}-{}", Ulid::generate())
    }

    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn keeps_query_order_per_owner() {
        let store = dynamo_store().await;
        let owner = unique_owner("query-order");

        for name in ["Running", "Water"] {
            store
                .create_action_type(
                    &owner,
                    NewActionType {
                        name: name.to_owned(),
                        unit: "km".to_owned(),
                        icon: "footprints".to_owned(),
                    },
                )
                .await
                .unwrap();
        }

        let types = store.list_action_types(&owner).await.unwrap();
        let names: Vec<&str> = types.iter().map(|one| one.name.as_str()).collect();
        assert_eq!(names, ["Running", "Water"]);
    }

    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn a_fetched_type_is_none_outside_its_owners_partition() {
        let store = dynamo_store().await;
        let owner = unique_owner("partition");
        let created = store
            .create_action_type(
                &owner,
                NewActionType {
                    name: "Running".to_owned(),
                    unit: "km".to_owned(),
                    icon: "footprints".to_owned(),
                },
            )
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
                .get_action_type(&owner, &created.id)
                .await
                .unwrap()
                .unwrap()
                .name,
            "Running"
        );
    }

    /// Exercises `condition_expression("attribute_exists(pk)")` and the
    /// `ConditionalCheckFailedException` branch — nothing in `Memory` models
    /// this at all, since a `HashMap` lookup has no separate conditional path.
    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn updating_a_stranger_to_the_partition_answers_none() {
        let store = dynamo_store().await;
        let owner = unique_owner("update-stranger");

        let result = store
            .update_action_type(
                &owner,
                "not-a-real-id",
                NewActionType {
                    name: "Jogging".to_owned(),
                    unit: "mi".to_owned(),
                    icon: "timer".to_owned(),
                },
            )
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn deleting_a_type_that_is_not_there_is_not_an_error() {
        let store = dynamo_store().await;
        let owner = unique_owner("delete-missing");

        store
            .delete_action_type(&owner, "not-a-real-id")
            .await
            .unwrap();
    }

    /// The action-record counterpart of `keeps_query_order_per_owner`, and
    /// what actually exercises `scan_index_forward(false)` — nothing in
    /// `Memory` has a separate code path for query direction, since it just
    /// reverses a `Vec`.
    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn lists_records_newest_first() {
        let store = dynamo_store().await;
        let owner = unique_owner("record-order");
        let action_type = store
            .create_action_type(
                &owner,
                NewActionType {
                    name: "Running".to_owned(),
                    unit: "km".to_owned(),
                    icon: "footprints".to_owned(),
                },
            )
            .await
            .unwrap();

        for value in [1.0, 2.0] {
            store
                .create_action_record(
                    &owner,
                    NewActionRecord {
                        type_id: action_type.id.clone(),
                        value,
                    },
                )
                .await
                .unwrap();
        }

        let listed = store.list_action_records(&owner).await.unwrap();
        let values: Vec<f64> = listed.iter().map(|record| record.value).collect();
        assert_eq!(values, [2.0, 1.0]);
    }

    /// Exercises `find_action_record`'s `Query`-then-match path — the
    /// approach this project chose over a secondary index for locating one
    /// record by its bare id (see the Work Log and Decision Record this
    /// choice produced).
    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn a_fetched_record_is_none_outside_its_owners_partition() {
        let store = dynamo_store().await;
        let owner = unique_owner("record-partition");
        let action_type = store
            .create_action_type(
                &owner,
                NewActionType {
                    name: "Running".to_owned(),
                    unit: "km".to_owned(),
                    icon: "footprints".to_owned(),
                },
            )
            .await
            .unwrap();
        let created = store
            .create_action_record(
                &owner,
                NewActionRecord {
                    type_id: action_type.id,
                    value: 5.2,
                },
            )
            .await
            .unwrap()
            .unwrap();

        assert!(
            store
                .get_action_record("someone-else", &created.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store
                .get_action_record(&owner, &created.id)
                .await
                .unwrap()
                .unwrap()
                .value,
            5.2
        );
    }

    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn updating_a_stranger_record_answers_none() {
        let store = dynamo_store().await;
        let owner = unique_owner("update-stranger-record");

        let result = store
            .update_action_record(&owner, "not-a-real-id", UpdateActionRecord { value: 1.0 })
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn deleting_a_record_that_is_not_there_is_not_an_error() {
        let store = dynamo_store().await;
        let owner = unique_owner("delete-missing-record");

        store
            .delete_action_record(&owner, "not-a-real-id")
            .await
            .unwrap();
    }

    /// What actually exercises `.limit()` on a real `Query` — `Memory`'s
    /// `.take()` has no equivalent code path, the same reasoning
    /// `lists_records_newest_first` gives for `scan_index_forward(false)`.
    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn recent_action_records_caps_at_ten_through_dynamo() {
        let store = dynamo_store().await;
        let owner = unique_owner("recent-cap");
        let action_type = store
            .create_action_type(
                &owner,
                NewActionType {
                    name: "Running".to_owned(),
                    unit: "km".to_owned(),
                    icon: "footprints".to_owned(),
                },
            )
            .await
            .unwrap();

        for value in 0..12 {
            store
                .create_action_record(
                    &owner,
                    NewActionRecord {
                        type_id: action_type.id.clone(),
                        value: f64::from(value),
                    },
                )
                .await
                .unwrap();
        }

        let recent = store.recent_action_records(&owner).await.unwrap();

        assert_eq!(recent.len(), 10);
        assert_eq!(recent[0].value, 11.0);
    }

    /// What actually exercises the `sk BETWEEN :from AND :to` key condition
    /// against a real `Query` — `Memory`'s filter has no equivalent code
    /// path.
    #[tokio::test]
    #[ignore = "needs `just dynamo` and `just dynamo-table`; see `just test-dynamo`"]
    async fn recent_summary_counts_todays_records_through_dynamo() {
        let store = dynamo_store().await;
        let owner = unique_owner("summary-window");
        let action_type = store
            .create_action_type(
                &owner,
                NewActionType {
                    name: "Running".to_owned(),
                    unit: "km".to_owned(),
                    icon: "footprints".to_owned(),
                },
            )
            .await
            .unwrap();

        for _ in 0..2 {
            store
                .create_action_record(
                    &owner,
                    NewActionRecord {
                        type_id: action_type.id.clone(),
                        value: 1.0,
                    },
                )
                .await
                .unwrap();
        }

        let summary = store.recent_summary(&owner).await.unwrap();

        assert_eq!(summary.total, 2);
        assert_eq!(summary.daily.len(), 10);
        assert_eq!(summary.daily[9], 2);
    }
}
