# DR-0030: DynamoDB-backed `Store` tests run opt-in, and the recipe that runs them is self-contained

Status: accepted
Date: 2026-08-16

## Context

DR-0018 gave `crates/server` two `Store` implementations; DR-0020 pinned
DynamoDB Local into the devcontainer so the `Dynamo` variant could be checked
without a deployment. Neither closed the gap DR-0020's own Consequences
section named plainly: "`cargo test` still covers only [`Memory`], and
nothing runs `dev-api-dynamo` automatically." Defining the automated test
strategy (`docs/design/testing.md`) raised the same gap again, and this time
the decision was to close it rather than restate it.

## Decision

`crates/server/src/store.rs` gained `dynamo_tests`, a module of
`#[tokio::test]`s that repeat a subset of the existing `Memory` assertions —
query order via `begins_with`, partition isolation, the conditional update's
`ConditionalCheckFailedException` branch, idempotent delete — against a real
`Store::Dynamo`. Every test is `#[ignore]`d and calls a `dynamo_store()`
helper that reuses `Store::from_environment()` rather than constructing
`Store::Dynamo` by hand, so what runs is the same selection code a deployment
takes, configured differently — the same principle DR-0020 states for
`dev-api-dynamo`. `dynamo_store()` asserts `TABLE_NAME` is set and panics
with a pointer to `just test-dynamo` if it is not, rather than silently
proceeding — the one failure mode that would make every test below pass
without checking anything is `Store::from_environment()` falling back to
`Memory` unnoticed.

The new `just test-dynamo` recipe is deliberately not a thinner version of
`dev-api-dynamo`'s pattern (start `dynamo`, run `dynamo-table`, expect both
already running in another terminal). It starts DynamoDB Local itself in the
background, polls readiness by retrying `just dynamo-table` for up to 30
seconds, runs `cargo test -p server -- --ignored`, and stops DynamoDB Local
again via `just dynamo-stop` in a `trap ... EXIT` — regardless of whether the
tests passed. One command, unattended.

## Alternatives

- **A Cargo feature flag gating the Dynamo tests, instead of `#[ignore]`.**
  Rejected: `#[ignore]` is the standard mechanism for "exists, does not run by
  default," needs no `Cargo.toml` change, and `cargo test -- --ignored` is
  already exactly what `test-dynamo` has to pass regardless.
- **Let `dynamo_store()` fall back to whatever `Store::from_environment()`
  returns, the way the production code does.** Rejected: unlike production,
  where `Memory` is a legitimate default, a test that silently runs against
  `Memory` because `TABLE_NAME` was not set is a test that reports success
  while checking nothing — worse than failing loudly, because nothing about a
  green `cargo test -- --ignored` would say so.
- **Background DynamoDB Local from the recipe and kill it by the job's own
  `$!` pid.** Rejected: `just dynamo` wraps the `java` invocation, and killing
  the wrapping process is not guaranteed to kill `java` underneath it
  cleanly. `dynamo-stop` already solves this exact problem — the devcontainer
  has no `ps`, `pkill`, `fuser` or `lsof` (workspace.md), so it finds the
  process by reading `/proc` for `comm = java` and a matching jar path in
  `cmdline` — so `test-dynamo` defers to it instead of re-deriving the same
  logic with a plain pid that might not point at the right process.
- **Require `just dynamo`/`just dynamo-table` running in another terminal
  first, the way `dev-api-dynamo` does.** Rejected for this use only:
  `dev-api-dynamo` is an interactive session a person watches and restarts by
  hand; `test-dynamo` is meant to run unattended, as one command, plausibly
  before a commit. `dev-api-dynamo` is unchanged and remains the manual,
  interactive check for the parts of the HTTP surface `dynamo_tests` does not
  assert.

## Consequences

The DynamoDB half of `Store` — the key encoding, the `begins_with` query, the
`AttributeValue` mapping, and the conditional-update error path DR-0020's own
words called out as compiled by every build and executed by nothing until
deployment — is now checked by one command, with no manual steps and no
second terminal. `just test`/`cargo test --workspace` is unaffected: the four
new tests are `#[ignore]`d, so neither Java nor a table is ever required by
the default path (`testing.md`'s constraint).

Each `dynamo_tests` test mints its owner from `Ulid::generate()` rather than
sharing a fixed name, so repeated runs against a table that outlives one
invocation cannot collide with each other or with a prior run's leftovers —
though `just dynamo`'s `-inMemory` flag already makes accumulation moot for
`test-dynamo`'s own usage, since the table does not survive DynamoDB Local
being stopped.

`dev-api-dynamo` and `dynamo_tests` now overlap in what they can catch for
the four behaviours the tests assert, but not in what they cover: the tests
check `Store` in isolation; `dev-api-dynamo` is still the only local check of
the full HTTP surface — routing, extraction, and the handlers in
`action_types.rs` — running on the DynamoDB store.
