# DynamoDB persistence: infrastructure and schema

Status: complete
Started: 2026-08-09
Branch: main

## Request

Persistence for this application is to be DynamoDB. As the first step, create
the infrastructure and the schema for it.

### Clarifications

Asked which table design to adopt, and which Terraform layer should own the
table.

The answer to the first is a **single table** holding both entities under
composite keys: partition key `USER#<cognito sub>`, sort key `TYPE#<id>` for
action types and `RECORD#<timestamp>#<id>` for action records, so that one
user's data is reachable by one query, and the table count, the IAM statements
and the published parameters stay a single set. The accepted sketch places
`name`, `unit`, `icon` and `created_at` on a type item, and `type_id`, `name`,
`unit`, `icon` and `value` on a record item, and reads the three known access
patterns — list types, ten most recent records, records within a ten-day window
— as sort-key queries against that one partition.

Asked whether copying an action type's display attributes onto each record item
is acceptable, given that it freezes the display of history against later edits
to the type. It is: the display of a past record is not required to follow a
later rename, because the record keeps the type's identifier, so what the record
*is* does not change — only the labels shown beside it are the ones that were in
force when it was recorded.

The answer to the second is a **new `infra/data` layer**, following the
blast-radius split of DR-0005, with its own state file and its own apply. The
table name is published through SSM; the `api` layer reads it for the Lambda's
IAM policy and environment. The apply order becomes bootstrap, delivery,
identity, data, api, and the destroy order its reverse.

## Interpretation

**What is being asked.** Stand up the persistence substrate: a Terraform layer
that creates the DynamoDB table, and a key and attribute design that the known
screens can actually be served from. The deliverable is infrastructure plus a
written schema, not a working data path.

**What is out of scope, for this unit of work.**

- The axum service still answers `GET /api/dashboard` from the hardcoded values
  in `crates/server/src/main.rs`. Replacing them with reads against the table,
  adding an AWS SDK dependency, and adding the write endpoints the action-type
  screens need are all later work.
- Extracting the Cognito `sub` from the validated JWT in the service. The schema
  assumes that identifier as its partition key; obtaining it is part of the data
  path, not of this step.
- Any change to `crates/shared`. Its types are the wire boundary between the SPA
  and the API; the persisted item shape is a separate concern and the two are
  deliberately not being unified here.
- Backup, point-in-time recovery tuning beyond a default, capacity tuning, and
  any second region.

**What is assumed.**

- The partition key is the Cognito `sub`, not the email address. `sub` is stable
  across profile changes; an email is not, and DR-0010's flow makes `sub` the
  identifier the API can trust from the token.
- On-demand billing (`PAY_PER_REQUEST`). There is one environment, no traffic
  baseline, and no reason to provision capacity.
- The table is as irreversible to destroy as the Cognito user pool, so it
  carries both `prevent_destroy` and DynamoDB's own deletion protection, which is
  the pattern `identity` already established for the pool.
- No global secondary index is created. Every access pattern the current design
  documents — the action-types list, the dashboard's ten most recent records, the
  ten-day window — is a sort-key query inside one user's partition. An index
  added speculatively costs storage and write throughput for a query nothing
  issues.
- Record items carry a copy of the type's `name`, `unit` and `icon` alongside
  `type_id`, per the accepted sketch. This makes a record self-sufficient at
  read time and pins the display of history to the values in force when it was
  recorded. It is a real trade-off — editing a type does not retroactively change
  past records — and it warrants a Decision Record rather than being left as an
  implementation detail.
- Sort keys sort lexicographically, so record timestamps are RFC 3339 in UTC
  with fixed width, which makes lexical order equal chronological order. The
  uniqueness suffix on both item kinds is a ULID, lexicographically sortable and
  generated without coordination.
- `infra/data` depends on nothing but `bootstrap`. It needs no value from
  `delivery` or `identity`; its position after `identity` in the apply order is
  a convention for reading, not a dependency.

## Plan

1. Write `infra/data` as a fourth root module in the established shape:
   `versions.tf`, `variables.tf`, `main.tf`, `outputs.tf`, `ssm.tf`,
   `backend.tf` with its own state key, and a committed `.terraform.lock.hcl`.
2. Define the table in `main.tf`: on-demand billing, `pk` and `sk` string keys,
   deletion protection, `prevent_destroy`, and server-side encryption left on
   the AWS-owned default.
3. Publish `/<project>/data/table_name` — and the table ARN, which the `api`
   layer needs for its IAM policy — as SSM parameters, matching how every other
   layer exposes itself (DR-0005).
4. Extend `infra/api`: read the two parameters, replace the
   basic-execution-only role with a policy granting the item operations the
   service will issue against that table and its keys alone, and pass the table
   name to the Lambda as an environment variable.
5. Add `data` to the `justfile` — the `tf-validate` loop and the apply-order
   comment — and to the same ordering note in `docs/design/deployment.md`.
6. Write the schema down as a durable document: the key encoding, the attributes
   of each item kind, and the query that serves each screen. Draft it as a new
   `docs/design/persistence.md` and have it confirmed before it lands, since a
   Design Document is overwritten rather than appended.
7. Draft the Decision Records this produces — the single-table design with its
   key encoding, and the denormalized type attributes on record items — and
   confirm the `docs/design/index.md` and `docs/design/deployment.md` updates
   that follow.
8. Verify with `just tf-fmt-check` and `just tf-validate`, then apply `data` and
   re-apply `api`.

## Progress

### 2026-08-09

Read `docs/README.md`, the Design Document index, `deployment.md`,
`page-layouts.md` and DR-0005 before planning. DR-0005 closes by naming a
database as the resource its layering scheme "most obviously anticipates and
does not yet have", so adding a fifth layer follows the decision already on
record rather than reversing it.

Confirmed the two open questions with the user: single-table design, and a new
`infra/data` layer. Both are recorded under Clarifications above.

Noted two decisions that will need Decision Records: the single-table key
encoding, and the choice to copy an action type's `name`, `unit` and `icon` onto
each record item.

Plan confirmed, including the denormalization above. Implemented steps 1 to 7.

`infra/data` written as the fifth root module, in the shape the other layers
use. The table is `<project>-app`: on-demand, `pk`/`sk` string keys, no
secondary index, `prevent_destroy`, `deletion_protection_enabled`, and
point-in-time recovery behind a variable defaulting to on. The last of these was
not in the plan and was added while writing it: `prevent_destroy` and deletion
protection both guard against destroying the *table*, and neither sees the
failure this store is actually likely to suffer — a bad write or a wrong delete
issued by the application. Continuous backups are the only guard that covers it.

`infra/api` extended: two SSM data sources, an inline role policy scoped to the
table ARN, and `TABLE_NAME` in the function's environment. `Scan` is left out of
the policy deliberately — every access pattern is a `Query` inside one user's
partition, so a `Scan` reaching this table would be a defect, and withholding
the permission turns that defect into an error rather than a silent full-table
read.

The policy cannot express per-user isolation. The function serves every user and
derives the partition key from the verified token, so its permissions necessarily
cover every partition; the isolation is in the service alone. This is recorded
as a constraint in the new Design Document because it is exactly the kind of
thing a reader assumes IAM is doing.

One thing the plan did not anticipate: `deployment.md` already said "State is
locked by S3's native lock file, not a DynamoDB table (DR-0006)". With a
DynamoDB table now in the stack, that sentence reads as a contradiction, so it
gained a clause distinguishing the two.

Wrote DR-0015 (single table keyed by owner and entity kind) and DR-0016 (records
copy their type's display attributes), and drafted `docs/design/persistence.md`
along with the `index.md` and `deployment.md` updates that follow. Those three
are overwrites and await confirmation.

Noticed but deliberately not changed: `docs/design/index.md` still describes the
backend as serving `GET /api/greeting`, which `crates/server` replaced with
`GET /api/dashboard`. It is unrelated to this work and is left for the user to
decide on.

**Step 8 is blocked.** The AWS session in this devcontainer has expired
(`login session has expired, please reauthenticate`), so neither `data` nor `api`
can be applied. Nothing else in the plan depends on it.

### 2026-08-09, later

The session was restored with `aws login` and step 8 ran. Both applies matched
what the plan above expected, with no surprise in either.

`data` planned three creates and applied them: the table in 10 seconds, then
both parameters. `api` planned one create and one in-place update — the inline
role policy, and `TABLE_NAME` joining the function's environment — and nothing
about the HTTP API, the authorizer or the function's code moved, which was the
one risk worth watching, since `just deploy-api` pushes that code outside
Terraform.

Two things the earlier entries got wrong, both now corrected:

The Verification section below claimed `just tf-validate` could not run end to
end because `bootstrap` and `api` carry `.terraform` directories from a real
backend `init`. That was the wrong diagnosis. `init -backend=false` does resolve
the stored backend in an initialised directory, but resolving it is not the
failure — the expired credentials were. With the session restored the recipe
runs clean over all five layers in place, with no copy and no directory removed.

`deployment.md` said `api` "cannot be planned until `data` has been applied
once". True at the time and now spent, so the paragraph is gone rather than
reworded; the note it sat under now says all five layers are applied and their
twelve parameters exist.

One detail the plan did not name: `describe-table` reports no `SSEDescription`
at all. That is the AWS-owned default key, which the table was left on
deliberately, and it is what an unset field means here — not encryption missing.
A reader checking the table by hand would otherwise read `null` as a defect.

The `index.md` staleness the previous entry left for the user to decide on was
confirmed and fixed in the same pass: the backend entry described
`GET /api/greeting` returning a `shared::Greeting`, which `crates/server`
replaced with `GET /api/dashboard` returning a `shared::Dashboard`, and the ci
entry counted four applied layers where there are now five.

## Verification

- `just tf-fmt-check` passes over the whole `infra` tree.
- `just tf-validate` passes over all five layers, in place. The earlier claim
  that it could not run end to end is retracted above: the cause was the expired
  session, not the leftover `.terraform` directories.

Applied and then checked against the account rather than against state:

- `describe-table` — `ACTIVE`, `PAY_PER_REQUEST`, `pk` HASH and `sk` RANGE both
  `S`, no global or local secondary index, `DeletionProtectionEnabled` true.
- `describe-continuous-backups` — point-in-time recovery `ENABLED`, 35 days.
- `just _ssm data/table_name` and `just _ssm data/table_arn` both resolve, and
  the project now publishes twelve parameters in total.
- The function's environment carries `TABLE_NAME=rust-leptos-axum-aws-app`.
- `get-role-policy` on the inline policy — the table ARN alone as the resource,
  eight item actions, no `Scan`.
- `GET /health` on the API endpoint still answers 200 `ok`, so the function
  starts after the environment change.
- `just tf-plan data` and `just tf-plan api` both report no differences, so
  configuration and account agree.

Not verified, because nothing exercises it yet: that the key encoding serves the
three screens. The table is empty and no code reads or writes it.

## Still to do

- The data path itself: an AWS SDK dependency in `crates/server`, the Cognito
  `sub` extracted from the validated token, and the endpoints the action-type
  screens need. None of it is part of this unit of work.

## Retirement

- [x] Design Documents updated — `persistence.md` written, `deployment.md` and
      `index.md` corrected to say the table exists
- [x] Decision Records written (DR-0015, DR-0016)
- [x] Non-obvious knowledge preserved — the rejected table-per-entity and
      speculative-index designs and the fixed-width `recorded_at` requirement in
      DR-0015, the denormalisation trade-off in DR-0016, and the constraints
      neither IAM nor the table can enforce in `persistence.md`
- [x] No durable document depends on this log
