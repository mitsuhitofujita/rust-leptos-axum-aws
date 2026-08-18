# DR-0032: An action record is located by owner-partition query, not a secondary index

Status: accepted
Date: 2026-08-17

## Context

`GET`/`PUT`/`DELETE /api/actions/{id}` each need to find one action record
from the bare id the API exposes. An action type's key, `TYPE#<ulid>`
(`persistence.md`), lets the id alone reconstruct the key, so those three
handlers were always a direct `GetItem`/`UpdateItem`/`DeleteItem`. An action
record's key, `RECORD#<recorded_at>#<ulid>`, does not have this property:
`<recorded_at>` sits between the prefix and the id, and the id alone cannot
reconstruct it.

Three shapes were weighed:

- **Query the owner's `RECORD#` range and match the trailing id**, the same
  `Query` `list_action_records` already runs, then operate on the one item
  whose key matches. No schema change.
- **Add a global secondary index keyed by the bare id**, giving a genuine
  point lookup. `infra/data/main.tf` currently carries an explicit comment
  committing to no secondary index ("an index costs storage and write
  throughput for queries nothing issues"), so this reverses a stated
  position rather than merely adding to it, and `id` would need to become
  its own top-level attribute — today it exists only encoded inside `sk`.
- **Drop `<recorded_at>` from the sort key**, making it `RECORD#<ulid>` like
  an action type's `TYPE#<ulid>`. Ordering is unaffected today only because
  `recorded_at` always equals creation time — no field to record a different
  one exists yet — but this would invalidate `persistence.md`'s
  already-designed (not yet implemented) dashboard window query, `sk BETWEEN
  "RECORD#<from>" AND "RECORD#<to>"`, which depends on the sort key carrying
  a human-meaningful, lexically-ordered date directly, and would silently
  foreclose backdating a record as a future feature.

Put to the user as three concrete options rather than decided unilaterally,
since each is a real trade-off with durable consequences. The user confirmed
the first.

## Decision

`find_action_record` (`crates/server/src/store.rs`) queries the owner's whole
`RECORD#` range with the same `begins_with(sk, "RECORD#")` condition
`list_action_records` uses, then finds the one parsed item whose id matches.
`get_action_record`, `update_action_record` and `delete_action_record` all
go through it for the `Dynamo` variant, reconstructing the full sort key from
the found record's own `recorded_at` and `id` before issuing the
`GetItem`-equivalent read, `UpdateItem` or `DeleteItem`. The `Memory` variant
needs none of this — a `Vec` search by id is already direct — so the two
variants' shapes diverge here for the first time.

The primary key schema in `infra/data/main.tf` and `docs/design/persistence.md`
is unchanged.

## Alternatives

- **A global secondary index keyed by the bare id.** Rejected for now, not
  permanently. It is the shape `persistence.md`'s own constraints anticipate
  ("adding an index is an online operation and needs no change to existing
  items" — DR-0015), and if get/update/delete-by-id becomes frequent at a
  scale where a per-operation partition query is measurably expensive, this
  is what should be reached for. It was not chosen now because nothing today
  demonstrates that cost, and paying the write-throughput and storage cost of
  an index ahead of a demonstrated need reverses a position `infra/data/main.tf`
  already states in its own comments.
- **Dropping `<recorded_at>` from the sort key.** Rejected because it trades
  a real problem in scope (locating one record by id) for breaking a
  different, already-designed piece of persistence the dashboard's window
  query depends on, and because it would remove the field's only reason to
  exist independent of the id — which forecloses backdating without that
  ever being a decision anyone made on its own terms.

## Consequences

Every single-record operation on `Dynamo` costs one `Query` across the
owner's full `RECORD#` range rather than one point read. This is bounded by
one person's total recorded actions, which this project's stated scale
(a personal habit tracker) keeps small; it is not bounded by anything else,
since the query is already the same one `list_action_records` runs with no
special optimisation.

This is explicitly revisited, not settled permanently: if per-owner record
counts or per-operation latency ever demonstrate this query is a real cost —
not "it could be a cost at some hypothetical scale" — the GSI alternative
above is what closes that gap, at the cost of reversing `infra/data/main.tf`'s
current no-secondary-index stance.
