# DR-0031: Router-level tests for `server` live inside `main.rs`, not a split `lib.rs`

Status: accepted
Date: 2026-08-16

## Context

`docs/design/testing.md` named a real gap: every existing test in
`crates/server` calls `validate()` or a `Store` method directly, so nothing
sends a request through the `Router` `main.rs` builds — a wrong HTTP method, a
route typo, a JSON body that fails to deserialize, or `Owner`/`State`
extraction ordering was not caught by `cargo test`.

Closing it with `tower::ServiceExt::oneshot` needs the `Router` construction
reachable from a test, and `crates/server` is a binary crate — `main.rs`
only, no `lib.rs` — so that construction lives inside `main()` where nothing
outside the process can reach it. Two shapes were considered:

- Factor `fn router(state: AppState) -> Router` out of `main()` and add
  `#[cfg(test)] mod tests` to `main.rs` itself, so the crate stays one binary
  target.
- Split into a `lib.rs` (the router, the state, the module declarations) and
  a thin `main.rs`, letting tests live under `tests/` as ordinary Cargo
  integration tests against `server` as a library.

The second is the shape most Cargo/axum documentation assumes for
"integration tests," and it is also this project's general default: asked to
weigh in, the reasoning was that a library boundary keeps tests honest about
only reaching what a crate actually makes `pub`, which is a real property
worth having as a rule. But a rule applied for its own sake here would be
solving "will code need this shape someday" rather than a need in front of
this decision — the crate has exactly one thing that would move into `lib.rs`
reaching for it (this one test module), and one reason to introduce a
structural split is not enough to introduce it. The general preference
stands; this decision is that the bar for spending it has not been met yet.

## Decision

`main.rs` grew `fn router(state: AppState) -> Router`, called by both
`main()` and a `#[cfg(test)] mod tests` added to the same file. `crates/server`
remains a single binary target — no `lib.rs`, no `tests/` directory.

## Alternatives

- **Split into `lib.rs` + a thin `main.rs`**, as above. Rejected for now, on
  its cost rather than its merit: it makes `server` a library nothing is
  meant to depend on — `workspace.md`'s Structure table describes it as "the
  axum API — compiled to the host target," not a library plus a binary, and
  states outright that "nothing else depends on anything else" beyond
  `shared`. Concretely, `tests/*.rs` compiles as a separate crate, so
  `testkey.rs` — documented as "not a secret," but still a fixture that
  exists only for tests to sign realistic tokens with — would have to become
  part of `server`'s public library API to stay reachable from it, or be
  duplicated. A larger diff for the same testing capability the chosen shape
  already gives.

## Consequences

Router-level tests (`tests::health_is_reachable_with_no_auth_at_all` and six
others in `main.rs`) exercise routing, extraction, and JSON (de)serialisation
against the real `Router`, in-process, with no listener and no network —
closing the gap `testing.md` named. `crates/server` stays exactly what
`workspace.md` already says it is: one binary, nothing depending on it as a
library.

The decision is explicitly reversible on new information: if a second,
independent reason to split into `lib.rs` shows up — not "the tests would be
tidier under `tests/`," which was already weighed here, but something the
current shape actually blocks — that is what revisits this record, not a
renewed preference for the shape rejected above.
