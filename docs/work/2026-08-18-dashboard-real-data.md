# Connect the dashboard to the real store

Status: complete
Started: 2026-08-18
Branch: main

## Request

Implement `GET /api/dashboard` against the real store instead of the fixed
values it currently answers with. `backend.md` and `persistence.md` already
describe this as deferred, separate work — this is that work.

## Interpretation

**What is being asked.** `crates/server/src/dashboard.rs` currently answers
`Dashboard { summary, recent }` from hardcoded values, with a comment saying
so. Replace its body with real queries against `Store`, per the two rows
`persistence.md`'s query table already specifies but marks unimplemented:

- Recent list: `pk = USER#<sub>` and `begins_with(sk, "RECORD#")`, descending,
  limit ten.
- Ten-day summary: `pk = USER#<sub>` and `sk BETWEEN "RECORD#<from>" AND
  "RECORD#<to>"`, with the per-day counts computed by the service — DynamoDB
  does no aggregation.

These are two separate queries with two separate limits, not one — both
`persistence.md` and `page-layouts.md` say so explicitly ("The list is not
required to contain exactly one record per chart bar").

**Out of scope:**
- Any frontend change. `crates/app/src/dashboard.rs` and `api.rs::fetch_dashboard`
  already consume the real `shared::Dashboard` shape; only the server's
  answer changes.
- Any change to the key schema, `infra/data/main.tf`, or the two-query
  approach itself — `persistence.md` already settled that shape, and DR-0032
  (adopted one day ago) explicitly reasoned about not breaking the sort key
  this query depends on.
- Retiring the two Work Logs still open (`2026-08-16-automated-test-strategy.md`,
  `2026-08-17-actions-crud.md`). They are unrelated to this request; not
  touched here.

**Assumption that is a real design gap, flagged rather than silently
resolved:** no design document states what a "day" is for the summary
window's bucketing. `recorded_at` is always UTC (`persistence.md`), and no
per-user timezone is captured anywhere in this system — Cognito, the store
schema, and `shared` all carry none. The only implementable reading today is
**a UTC calendar day**: "today" is the current UTC date, and the window is
that date and the nine before it. A visitor in a timezone far from UTC will
see their "today" bar roll over at UTC midnight rather than their own local
midnight. This is a real trade-off with a real alternative (capturing a
per-user timezone, or defining the window as a rolling 24×10-hour span from
the request instant instead of calendar-aligned days) that nothing wrote
down — worth a Decision Record when this closes, not a silent choice.

**Implementation shape assumed** (naming/reuse calls, not requirements):
- Two new `Store` methods, matching the granularity of `persistence.md`'s
  query table and the existing one-method-per-query-row pattern
  (`list_action_types`, `get_action_type`, …): `Store::recent_action_records`
  (the capped list) and `Store::recent_summary` (the windowed, bucketed
  count), the latter returning `shared::RecentSummary` directly — `Store`
  already returns wire types straight from its other methods (e.g.
  `create_action_type` returns `ActionType`), so this is not a new pattern.
- `Store::list_action_records` (no cap, used by `/api/actions`) and the new
  `Store::recent_action_records` (capped at ten) share the same DynamoDB
  query shape apart from `.limit()`; plan is to factor the common part into
  a private helper rather than duplicate the query builder chain, since the
  duplication would otherwise be near-total.
- Day-boundary comparison uses string comparison against the same
  fixed-width `RECORD#<recorded_at>` encoding the rest of `store.rs` already
  relies on for lexical-equals-chronological order, rather than parsing
  timestamps back out — `time` is only pulled in with the `formatting` and
  `macros` features (no `parsing`), so this also avoids a `Cargo.toml`
  change.
- `dashboard.rs` gets a minimal `Failure` (just the store-unavailable case,
  answering `500`) mirroring `actions::Failure`/`action_types::Failure`'s
  existing per-module pattern — there is nothing to validate and nothing to
  locate by id, so it has no other variant.

## Plan

1. `crates/server/src/store.rs`:
   - Factor `list_action_records`'s DynamoDB query into a private helper
     that takes an optional limit; `list_action_records` calls it with
     `None`, a new `recent_action_records` calls it with
     `Some(RECENT_RECORDS_LIMIT)`. Update the `Memory` arm the same way.
   - Add `recent_summary`: compute today's UTC date and the nine-day-earlier
     start, format the ten day-boundaries (plus the exclusive upper bound)
     with the existing `TIMESTAMP` format, run the `BETWEEN` query (`Dynamo`)
     or an equivalent filter (`Memory`), then bucket the matched records into
     a ten-element `Vec<u32>` by comparing each record's `recorded_at`
     against the boundaries. Return `RecentSummary { total, daily }`.
   - Small refactor: extract `now()`'s
     `OffsetDateTime::now_utc().format(TIMESTAMP)` into a `format_instant`
     helper so `recent_summary`'s day-boundary formatting reuses it instead
     of repeating the `.format(TIMESTAMP)` call and its error mapping.
2. `crates/server/src/dashboard.rs`: replace the hardcoded body with calls to
   `store.recent_summary` and `store.recent_action_records`, assembled into
   `Dashboard`. Add the module's `Failure` type. Update the module doc
   comment, which currently says "still answering from fixed values".
3. `crates/server/src/main.rs`: update the top-of-file doc comment that says
   `/api/dashboard` still answers from fixed values.
4. Tests:
   - `store.rs`: `recent_action_records` caps at ten and stays newest-first
     with more than ten stored; `recent_summary` on an empty partition
     answers ten zero buckets and `total: 0`; `recent_summary` after
     creating a few records today answers `total` equal to the count, all of
     it in the last (today) bucket, and partition isolation (another
     owner's records do not contribute).
   - `main.rs` router-level test: create an action type, record a couple of
     actions through `/api/actions`, `GET /api/dashboard` through the router,
     and assert the response's `recent` and `summary.total` reflect what was
     actually stored — the router-level counterpart the existing actions
     lifecycle test already established the pattern for.
5. `just check`, `just lint`, `just test` (and, if available, `just
   test-dynamo`, since the new queries add a `BETWEEN` and a `.limit()` this
   project's DynamoDB-Local tests have not exercised before — worth at least
   one opt-in test alongside the existing `dynamo_tests` pattern if time
   permits, though `persistence.md`'s constraint list does not require it
   for every query).
6. Present the day-boundary assumption as a Decision Record candidate at
   work-done time, and update `backend.md`/`persistence.md`/`page-layouts.md`
   to drop their "still hardcoded" / "not yet implemented" language.

## Progress

### 2026-08-18

Implemented per the plan, with one refinement discovered along the way:

- `crates/server/src/store.rs`: `list_action_records` and the new
  `recent_action_records` now share a private `query_records(owner, limit:
  Option<i32>)` helper rather than duplicating the query builder chain.
  Added `recent_summary`, computing the UTC day boundaries with
  `Date::midnight().assume_utc()` and `time::Duration::days`, then bucketing
  matched records by comparing fixed-width `recorded_at` strings against
  those boundaries — no timestamp parsing, and no `Cargo.toml` change, since
  `time` only has the `formatting`/`macros` features enabled. Extracted
  `now()`'s formatting call into a shared `format_instant` helper reused by
  the day-boundary formatting.
- `crates/server/src/dashboard.rs`: rewritten to call `store.recent_summary`
  and `store.recent_action_records` and assemble `Dashboard`. Added a
  minimal `Failure` (store-unavailable only, answering `500`), mirroring
  `actions::Failure`.
- `crates/server/src/main.rs`: updated the top-of-file doc comment (no
  longer says `/api/dashboard` answers fixed values); added a router-level
  test, `dashboard_reflects_recorded_actions_through_the_router`, recording
  two actions through `/api/actions` and asserting `GET /api/dashboard`
  reflects them.
- Tests added in `store.rs`: three `Memory`-backed unit tests
  (`recent_action_records_caps_at_ten_newest_first`,
  `recent_summary_of_an_empty_partition_is_ten_zero_buckets`,
  `recent_summary_counts_todays_records_into_the_last_bucket`,
  `recent_summary_does_not_count_another_owners_records`) and two
  `#[ignore]`d `dynamo_tests` (`recent_action_records_caps_at_ten_through_dynamo`,
  `recent_summary_counts_todays_records_through_dynamo`) — added beyond the
  original plan's "if time permits," once it was clear `.limit()` and a
  `BETWEEN` key condition are both DynamoDB-only code paths `Memory` has no
  equivalent for, the same reasoning `testing.md` already gives for why
  `scan_index_forward(false)` needed its own `dynamo_tests` entry.
- `just fmt` was needed after the edits (two-line wraps `rustfmt` prefers
  differently than first written); `just fmt-check` is clean now.

No deviation from the plan otherwise. The UTC-calendar-day assumption from
Interpretation was implemented as stated and not revisited.

## Verification

- `just test`: 64 passed, 0 failed, 10 ignored (was 59/0/8 before this work:
  +5 in the default run — four new `Memory`-backed `store` tests and the new
  `dashboard_reflects_recorded_actions_through_the_router` router-level test
  — and +2 ignored `dynamo_tests`).
- `just test-dynamo`: 10 passed, 0 failed, against a real DynamoDB Local —
  including the two new tests, which is what actually confirms `.limit()`
  and `sk BETWEEN :from AND :to` work against the real `Query` API, not just
  compile against it.
- `just lint` (`cargo clippy --workspace --all-targets -- -D warnings` and
  the `app`/wasm32 pass): clean.
- `just fmt-check`: clean.

## Retirement

- [x] Design Documents updated — `backend.md`, `persistence.md`, `testing.md`,
      `index.md`. `deployment.md` deliberately untouched: it records deployed
      reality, and this change has not been deployed.
- [x] Decision Records written (DR-0033)
- [x] Non-obvious knowledge preserved — the UTC-calendar-day boundary and its
      rejected alternatives are in DR-0033; the bucketing method (string
      comparison, not parsing) is in `persistence.md`; the hand-maintained
      `RECENT_RECORDS_LIMIT`/`SUMMARY_WINDOW_DAYS` duplication is a
      Consequence in DR-0033.
- [x] No durable document depends on this log — confirmed by grep; nothing
      under `docs/design/` or `docs/decisions/` names this file.

Kept past completion at the user's request, until the branch merges — see
`docs/README.md`'s retirement checklist: the checklist passing is what
matters, deletion is bookkeeping.
