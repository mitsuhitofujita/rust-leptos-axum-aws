# DR-0015: One DynamoDB table holds every entity, keyed by owner and entity kind

Status: accepted
Date: 2026-08-09

## Context

Nothing has been persisted so far. `GET /api/dashboard` answers with values
hardcoded in `crates/server`, deliberately, so that the boundary types and the
fetch path could be real before the data was. The screens in
`docs/design/page-layouts.md` describe two entities — an action type, which
supplies a name, a numeric unit and a Lucide icon name, and an action record,
which pairs one type with a numeric value and an instant — and every screen that
displays either is scoped to the signed-in user.

DynamoDB was chosen as the store. That choice determines almost nothing on its
own: what has to be decided is the key design, because in DynamoDB the keys are
the query language. Three access patterns are already known and none of them is
hypothetical:

- the action-types screen lists every type the user has registered;
- the dashboard shows the ten most recent records, newest first;
- the dashboard summarises a ten-day window, one count per day.

An `Actions` screen listing recorded actions is in the design but its filtering
and paging are unspecified.

Two further facts shape the answer. Every one of these queries is scoped to one
user, and the user is identified by the Cognito `sub` carried in the access
token the JWT authorizer has already validated (DR-0010). And the infrastructure
is layered by blast radius (DR-0005), so each additional table is another set of
resources, another IAM statement, and another published parameter.

## Decision

**One table, `<project>-app`, with a two-attribute composite key.** The
partition key `pk` is the owner. The sort key `sk` carries the entity kind as a
prefix followed by whatever that kind orders by:

| Item | `pk` | `sk` |
| --- | --- | --- |
| Action type | `USER#<cognito sub>` | `TYPE#<ulid>` |
| Action record | `USER#<cognito sub>` | `RECORD#<recorded_at>#<ulid>` |

Everything one user owns therefore lives in one partition, and each access
pattern is a sort-key condition inside it: `begins_with "TYPE#"` lists the
types, `begins_with "RECORD#"` in descending order with a limit of ten gives the
dashboard list, and a `BETWEEN` over two `RECORD#` bounds gives the ten-day
window.

**No secondary index.** Every pattern above is answered by the primary key.

**On-demand billing**, and a table as guarded as the Cognito user pool —
`prevent_destroy` against Terraform, DynamoDB's deletion protection against the
console and CLI, and point-in-time recovery for the application-level mistake
that neither of those sees.

The concrete key encoding, attribute list, and the query behind each screen are
in `docs/design/persistence.md`.

## Alternatives

**A table per entity — `action-types` and `action-records`.** The obvious
translation of a relational schema, and the strongest alternative. Rejected on
cost rather than on correctness: it doubles the resources, the IAM statements,
the published parameters and the environment variables for two entities that are
always read by the same request, on behalf of the same user, and it buys nothing
back, because DynamoDB has no join to lose. The readability it offers is real,
and `docs/design/persistence.md` is what replaces it.

**A relational database — RDS or Aurora Serverless.** Rejected before this
decision rather than within it, but worth recording. The access patterns are
few, known, and key-shaped; the workload is idle whenever nobody is using the
SPA; and a VPC-attached database would put the Lambda in a VPC, which the
current `api` layer deliberately avoids. Aurora Serverless v2 does not scale to
zero, so it is a standing cost for a project with no users yet.

**Index overloading — a generic `gsi1pk`/`gsi1sk` pair added now.** Rejected as
speculative. An index costs storage and consumes write capacity on every write,
and no query today needs one. The pattern most likely to want one — listing
records of a single type across time — is not in the current design, and adding
an index later is an online operation that requires no change to existing items.

**A composite sort key without the entity prefix**, relying on separate
partitions such as `USER#<sub>#TYPES`. Rejected because it splits one user
across partitions for no gain and gives up the option of fetching a user's
entire state in one query.

**Email as the partition key.** Rejected: an email address changes and `sub`
does not, and a key that changes is not a key. `sub` is also what the token
carries without any additional lookup.

## Consequences

Easy: one table to create, one ARN to grant, one name to pass; a user's whole
dataset is one query away, which makes an export or a delete-my-account
operation a single `Query` and a batch of deletes; and the ten-day window and
the ten most recent records — separate limits by design — are two conditions
over the same key rather than two schemas.

Hard, and accepted:

- **The sort key's ordering is lexical, so the timestamp format is load-bearing.**
  `recorded_at` must be fixed-width RFC 3339 in UTC, or chronological and lexical
  order diverge and the dashboard silently shows the wrong ten records. This is a
  constraint on the writer that nothing in the table enforces.
- **Every access pattern not answerable by the primary key needs a new index**,
  and index design is where the cost of a single table concentrates. The first
  query that wants records of one type across time is the case to expect.
- **All of one user's items share a partition.** DynamoDB's per-partition limits
  are far above anything one person generates, so this is a theoretical ceiling
  rather than a practical one, and there is no item-collection size limit to
  worry about because the table has no local secondary index.
- **The item shape is invisible to the type system.** `crates/shared` describes
  the wire, not the store, and nothing checks that a written item matches what a
  reader expects. Keeping the encoding in one documented place is the only
  guard.
- **Reversing this is a migration, not a refactor.** Splitting into two tables
  later means reading every item and rewriting it elsewhere. That is cheap while
  the table is empty and grows with use, so the moment to revisit it is before
  there are users.
