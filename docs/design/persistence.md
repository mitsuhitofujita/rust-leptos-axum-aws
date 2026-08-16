# Persistence

Updated: 2026-08-11

Note: action types are stored. `crates/server` reads and writes the `TYPE#`
half of this schema, and derives the partition key from the Cognito `sub` in the
`AuthContext` the edge produced (DR-0024). Action records are not — `GET /api/dashboard`
still answers from hardcoded values, so nothing has yet written a `RECORD#`
item. Everything below about records describes the store the service is being
written against; everything about action types describes what it does.

## Purpose

Where application data lives, how it is keyed, and which query serves each
screen.

There is one store: a single DynamoDB table holding both entities in the design —
the action type and the action record — partitioned by the user who owns them
(DR-0015). Everything about that table's shape follows from three access
patterns and no others; `docs/design/page-layouts.md` is where they come from.

## Structure

### The table

`rust-leptos-axum-aws-app`, created by the `data` Terraform layer. On-demand
billing, point-in-time recovery on, a primary key of `pk` and `sk`, and no
secondary index.

`just dynamo-table` creates a table of the same name and key schema in a local
DynamoDB, for verifying the service against the store it is written for without
deploying it (DR-0020). That copy of the schema and `infra/data/main.tf` are both
copies of what this document defines, and are kept in step with it by hand.

Only the two key attributes are declared to DynamoDB. Every other attribute
belongs to the application, so adding one is not an infrastructure change.

Encryption at rest is DynamoDB's AWS-owned default, so `infra/data/main.tf`
carries no `server_side_encryption` block. `describe-table` reports that state by
omitting `SSEDescription` altogether: its absence means the default key, not the
absence of encryption.

### Key encoding

```text
pk = USER#<cognito sub>
sk = TYPE#<ulid>                        an action type
     RECORD#<recorded_at>#<ulid>        an action record
```

`#` is the delimiter and appears in no component.

**`<cognito sub>`** is the subject claim of the access token the JWT authorizer
has already validated (DR-0010). It is the identity the service may trust
without a lookup, and unlike an email address it does not change.

**`<ulid>`** is a ULID: 26 characters of Crockford base32, fixed width,
lexicographically ordered by creation time, and generable without coordination.
It is the entity's identifier and is what the API exposes as `id`; the `TYPE#`
and `RECORD#` prefixes are storage encoding and never appear on the wire.

**`<recorded_at>`** is RFC 3339 in UTC at fixed width — `2026-08-08T07:12:00.000Z`,
always a `Z` offset and always three fractional digits. The sort key orders
lexically, so a variable-width instant orders wrongly; the fixed format is what
makes lexical order equal chronological order. The ULID after it breaks ties
between records written in the same millisecond and keeps the key unique.

### Items

**Action type** — `sk` = `TYPE#<ulid>`

| Attribute | Type | Meaning |
| --- | --- | --- |
| `name` | S | The label shown on records, e.g. `Running` |
| `unit` | S | The unit shown beside every value, e.g. `km` |
| `icon` | S | A canonical kebab-case Lucide name, e.g. `person-standing` — DR-0014 |
| `created_at` | S | Fixed-width RFC 3339 UTC |

**Action record** — `sk` = `RECORD#<recorded_at>#<ulid>`

| Attribute | Type | Meaning |
| --- | --- | --- |
| `type_id` | S | The bare ULID of the action type — what the record *is* |
| `name` | S | The type's name as it was when the record was written |
| `unit` | S | The type's unit, likewise |
| `icon` | S | The type's icon, likewise |
| `value` | N | The recorded number, one field for every unit |
| `recorded_at` | S | The same instant that is embedded in `sk` |

The three copied attributes are deliberate duplication: a record renders from
itself, and editing or deleting its type does not rewrite history. `type_id` is
what preserves the record's identity across a rename — DR-0016.

`recorded_at` is stored as an attribute as well as inside the key so that a
reader never has to parse the key to display an item.

### The query behind each screen

| Screen | Query |
| --- | --- |
| Action types list | `pk = USER#<sub>` and `begins_with(sk, "TYPE#")` — implemented |
| Edit action type | `GetItem` on `pk`, `sk = TYPE#<id>` — implemented |
| Create a type | `PutItem` on that key — implemented |
| Edit / delete a type | `UpdateItem`, `DeleteItem` on that key — implemented |
| Dashboard, ten recent records | `pk = USER#<sub>` and `begins_with(sk, "RECORD#")`, descending, limit 10 |
| Dashboard, ten-day summary | `pk = USER#<sub>` and `sk BETWEEN "RECORD#<from>" AND "RECORD#<to>"` |
| Add action | `PutItem` |

Descending order is `ScanIndexForward = false`; DynamoDB applies the limit after
ordering, so the ten newest cost one read of ten items rather than a scan of the
partition.

In the window query, `<from>` is the first instant of the window and `<to>` is
the first instant *after* it. `BETWEEN` is inclusive at both ends, but every
stored key has a `#<ulid>` suffix and therefore sorts strictly after the bare
`RECORD#<to>` bound, which makes the upper end exclusive without an
off-by-one-millisecond adjustment. The per-day counts are computed by the
service from the returned items; DynamoDB does no aggregation.

The dashboard's two limits — a ten-day window and ten records — are separate, as
`docs/design/page-layouts.md` requires. They are two conditions over one key, not
two schemas.

### The `data` layer

```text
infra/data/
  versions.tf     provider pin, default tags
  variables.tf    project, region, point_in_time_recovery
  main.tf         the table
  outputs.tf      table_name, table_arn
  ssm.tf          both, published for the api layer
  backend.tf      state key data/terraform.tfstate
```

It is its own root module because destroying it is irreversible, which is the
axis the infrastructure is divided along (DR-0005). It reads nothing from
another layer, so it depends on `bootstrap` alone; the create order is
`bootstrap`, `delivery`, `identity`, `data`, `api`, and `data` sits fourth for
readability rather than for any dependency.

## Interfaces

### What this layer publishes

| Parameter | Written by | Read by |
| --- | --- | --- |
| `/<project>/data/table_name` | `data` | `api`, as the Lambda's `TABLE_NAME` |
| `/<project>/data/table_arn` | `data` | `api`, to scope the Lambda's IAM policy |

Both travel through SSM Parameter Store rather than through state, like every
other cross-layer value (DR-0005).

### What the service is granted

The `api` layer attaches an inline policy to the Lambda's execution role
covering `GetItem`, `BatchGetItem`, `Query`, `PutItem`, `UpdateItem`,
`DeleteItem`, `BatchWriteItem` and `TransactWriteItems`, on the table ARN alone.

`Scan` is deliberately absent. Every access pattern above is a `Query` inside one
user's partition, so a `Scan` against this table would be a defect; withholding
the permission turns that defect into an error instead of a full-table read.

The table name reaches the function as the `TABLE_NAME` environment variable.
`crates/server` reads it at startup, and its absence is what selects the
in-memory store development runs on — so an unset variable is a working service
with no table rather than a broken one (DR-0018). See
[Backend](backend.md).

## Constraints

- **The partition key is the Cognito `sub`, and the service derives it from the
  validated token rather than from anything the client sends.** The IAM policy
  cannot express this — the function serves every user, so its permissions cover
  every partition. User isolation lives entirely in the service, and a handler
  that takes a user id from a request parameter defeats it — DR-0010.

- **`recorded_at` must be fixed-width RFC 3339 in UTC.** Lexical order is the
  only order the sort key has. A variable-width instant, a local offset, or a
  missing fractional part puts records in the wrong sequence, and the failure is
  silent: the dashboard shows ten records, just not the ten newest — DR-0015.

- **A record's `name`, `unit` and `icon` are historical values and are never
  refreshed from the type.** Editing a type changes future records only. Nothing
  reconciles the two, because they answer different questions — DR-0016.

- **`icon` is a canonical kebab-case Lucide name from the supported catalog**,
  never free text and never markup. The service checks a proposed name against
  `shared::icon_names` before storing it, and an unknown name that is already
  stored falls back to a generic glyph in the frontend rather than rendering
  nothing — DR-0012, DR-0014, DR-0019.
- **`name` and `unit` are stored trimmed, non-empty, and length-limited.** The
  limits are the service's, in `crates/server/src/action_types.rs`; no document
  fixes them, and nothing in the table enforces them.

- **There is no secondary index, so any access pattern not answered by the
  primary key requires one.** Listing every record of a single action type
  across time is the first such query to expect; it is not in the current design.
  Adding an index is an online operation and needs no change to existing items —
  DR-0015.

- **The table is guarded twice and backed up.** `prevent_destroy` stops
  Terraform, `deletion_protection_enabled` stops the console and the CLI, and
  point-in-time recovery covers the case neither sees: a bad write or a wrong
  delete issued by the application itself.

- **`crates/shared` describes the wire, not the store.** Its `ActionType` and
  `ActionRecord` are what the SPA and the API exchange; the item shape above is
  separate and deliberately not unified with them, so a stored attribute can be
  added without changing the API contract. Nothing type-checks one against the
  other.

- **The API exposes the bare ULID as an id, never the encoded key.** A client
  that received `TYPE#01J...` would be holding a storage detail, and the encoding
  could not then change without breaking it.

- **All of one user's items share a partition.** This is what makes a
  whole-account export or delete one query, and it means per-user throughput is
  bounded by DynamoDB's per-partition limits — far above what one person
  generates. There is no item-collection size limit, because the table has no
  local secondary index.
