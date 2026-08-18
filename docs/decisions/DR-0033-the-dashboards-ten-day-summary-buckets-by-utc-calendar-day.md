# DR-0033: The dashboard's ten-day summary buckets by UTC calendar day

Status: accepted
Date: 2026-08-18

## Context

`GET /api/dashboard`'s ten-day summary assigns each matched action record to
one of ten daily buckets, oldest first (`persistence.md`). "One day" needs a
boundary, and nothing in the system states what a day is from the visitor's
point of view: `recorded_at` is always stored and exchanged as UTC
(`persistence.md`), and no per-user timezone is captured anywhere — not in
the DynamoDB item shape, not in `shared`, not in the Cognito claims the
service reads.

## Decision

`Store::recent_summary` (`crates/server/src/store.rs`) buckets by UTC
calendar day: "today" is `OffsetDateTime::now_utc().date()`, and the window
is that date and the nine before it. A visitor whose local day does not
align with UTC sees the "today" bar — and the total it feeds — roll over at
UTC midnight, not their own local midnight.

## Alternatives

- **Capture a per-user timezone** (at sign-up, or inferred from the
  browser) and bucket in it. Rejected for now: nothing in this system
  stores any per-user preference today, `shared::ActionRecord`/the
  DynamoDB item shape would both need a new field, and no design document
  or mockup asks for one — it would be new scope invented to answer a
  question this work only found because it went looking, not one anyone
  has asked for.
- **A rolling window**: ten 24-hour buckets counted back from the request
  instant, rather than calendar-aligned days. Rejected because it does not
  actually answer the underlying question — it still has to decide what
  "24 hours ago from now" means for grouping — and because the ten bars are
  meant to correspond to something a visitor recognizes as "days," which a
  window that drifts with the request instant does not track either, once
  that instant moves away from local midnight.

## Consequences

Simple, and consistent with the only timezone this system already keeps
records in — no schema change, no new field. Costs some accuracy for
visitors far from UTC: their "today" bar can under- or over-count relative
to their own calendar day, most visibly in the hours after their local
midnight but before UTC's. Reversing this means adding a per-user timezone
somewhere the identity or the record schema does not have one today, which
is a real schema change, not a one-line fix.

Also worth naming: `RECENT_RECORDS_LIMIT` and `SUMMARY_WINDOW_DAYS`
(`crates/server/src/store.rs`) are each a hand-maintained copy of a count
`page-layouts.md` and `shared::RecentSummary`'s own doc comment already
state in prose — "capped at ten" and "Exactly ten counts," respectively.
Nothing type-checks the constant against the document; changing either cap
needs both edited by hand, the same unenforced-mirroring shape this project
already accepts for `infra/data/main.tf`'s local DynamoDB table copy.
