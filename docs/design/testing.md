# Testing

Updated: 2026-08-16

## Purpose

What automated verification exists for this workspace, what each layer is for,
and what deliberately stays manual. `cargo check`/`clippy` (see
[workspace.md](workspace.md)) catch what does not compile; this document is
about what compiles but could still be wrong.

It covers the Rust workspace — `crates/shared`, `crates/server`, `crates/app`
— and, briefly, `infra/`. It does not re-argue the manual verification recipes
DR-0020 and DR-0028 already established for what needs real AWS; it says which
gaps those recipes leave and what closes them without one.

This document was drafted collaboratively and awaits the human confirmation
Design Documents need before an update is considered final (`docs/README.md`).
One item remains explicitly deferred rather than decided — see "Proposed: pure
logic in `crates/app`" — everything else in Structure describes what is
implemented today.

## Structure

### Today

Every test in the workspace is a plain `#[test]`/`#[tokio::test]` inside a
`#[cfg(test)] mod tests` next to the code it checks — no `tests/` directory,
no dev-dependency beyond what a handful of async tests need from `tokio` and
`tower`, and no mocking crate anywhere. `just test` is `cargo test --workspace`.

| Crate | Tests | What they check |
| --- | --- | --- |
| `shared` | 0 | — |
| `server` | 45 run by default, 4 `#[ignore]`d | validation rules (`action_types.rs`); routing, extraction and (de)serialisation through the real `Router` (`main.rs`); the in-memory `Store` and, opt-in, the DynamoDB one (`store.rs`); Cognito token verification end to end — signature, issuer, audience, expiry, `kid` lookup (`cognito.rs`, `jwks.rs`, `identity.rs`) |
| `app` | 0 | — |

`server`'s auth tests are the pattern worth keeping: `testkey.rs` is a
committed RSA fixture (`#[cfg(test)]` only) that lets a test sign a genuinely
well-formed token and then vary one claim, with the verifier's clock passed in
as an argument so nothing depends on the date it runs. Nothing is mocked —
`cognito::verify` runs against a real signature over a real (if fixture) key
set. New tests in `server` should keep reaching for a real collaborator
(`Store::Memory`, `testkey.rs`) before reaching for a mock.

**Router-level tests.** `main.rs`'s `#[cfg(test)] mod tests` drives the real
`Router` with `tower::ServiceExt::oneshot` — in-process, no listener, no
network — against `Store::Memory` and both `Auth::Mock` and `Auth::Cognito`
(signed with `testkey.rs`). It covers what nothing else did: a full
create-then-list round trip through routing, extraction and JSON
(de)serialisation; a malformed JSON body answering axum's own `400` rather
than reaching `action_types::validate` at all; an unmapped method answering
`405`; and both auth arrangements' `401`s reaching the visitor as an actual
HTTP response, not just as what the extractor function returns in isolation.
The construction lives inside `main.rs` rather than a split `lib.rs` —
DR-0031.

**Opt-in: the DynamoDB half of `Store`.** `store::dynamo_tests` repeats a
subset of the `Memory` assertions — query order via `begins_with`, partition
isolation, the `attribute_exists(pk)` conditional update, idempotent delete —
against a real `Store::Dynamo`, reusing `Store::from_environment` so the
selection code path is the one a deployment actually takes (DR-0020). Every
test is `#[ignore]`d, so `cargo test --workspace` never touches Java; `just
test-dynamo` starts DynamoDB Local itself, waits for it, creates the table,
runs `cargo test -p server -- --ignored`, and stops DynamoDB Local again on
the way out (via `just dynamo-stop`, whether the tests passed or not) — one
command, unattended, no second terminal the way `dev-api-dynamo` needs one.
`just dev-api-dynamo` remains the manual, interactive check for everything
these tests do not assert (DR-0030).

One thing a request goes through is still not exercised by any of this:

- **Anything in `crates/app`.** Zero tests today. `app.rs`'s router, the
  screens, and `auth.rs`'s PKCE flow are checked only by running `dev-web` /
  `dev-web-auth` in a browser — the devcontainer has neither a browser nor
  Node.js (workspace.md's constraints), so this is not a gap that closes on
  its own.

### Proposed: pure logic in `crates/app`, unit-tested on the host target

`cargo test --workspace` already compiles `crates/app` for the host target
(confirmed: it runs today, reporting zero tests) — a plain `#[test]` on
non-web-sys code needs no `wasm-bindgen-test` and no browser. The frontend has
more of this than its zero tests suggest:

- `auth.rs`'s PKCE challenge is one line —
  `URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))` — inline
  inside `begin_sign_in`, which also calls `web_sys::Crypto`. Factored into a
  free function taking the verifier as a byte slice, it is pure and directly
  testable against RFC 7636's own test vector.
- `decode_jwt_claims` (base64url + JSON, no DOM) and the token-expiry
  comparison it feeds are already pure or nearly pure as written.
- `hosted_ui`/`params` build URLs from string inputs and could be tested the
  same way once separated from the `Window`/`Storage` calls around them.

**Deferred, not scoped into this pass.** Extracting these functions touches
`auth.rs`'s structure, not just its test coverage, and the discussion that
produced this document chose to take that on separately rather than as part
of defining the strategy.

What stays out of reach regardless: anything that actually calls `web_sys` —
`showModal`, `sessionStorage`, `fetch`, the router rendering a screen. Closing
that gap would mean `wasm-bindgen-test` plus a headless browser, which the
devcontainer image does not have today (workspace.md) and which this document
does not propose adding — the same reasoning DR-0023 gives for not
reproducing a whole further capability just to make one thing checkable.
DOM-dependent behaviour stays a manual check via `dev-web`/`dev-web-auth`.

### `crates/shared`

Serde round-trip tests for `ActionType`/`NewActionType`/`Dashboard` are cheap
to add but low-value today: the workspace shares these types at compile time,
so `app` and `server` cannot disagree about a field without a build failure.
The risk a round-trip test would catch — an unintentional wire-shape change —
is real but minor enough that this document treats it as optional, not
proposed.

### `infra/`

`just tf-fmt-check` and `just tf-validate` already exist, need no AWS
credentials and no backend, and run against an unapplied tree — nothing here
proposes more than what those two already do. Real correctness of a layer is
still only checked by `tf-plan`/`tf-apply` against the one live environment,
which DR-0005's single-environment reality and the 2026-08-16 retrospective's
Problem section both already name as a cost this project accepts rather than
one this document tries to remove.

## Interfaces

| Command | Runs | Status |
| --- | --- | --- |
| `just test` | `cargo test --workspace` | exists |
| `just test-dynamo` | DynamoDB Local (self-started) + table + `cargo test -p server -- --ignored`, cleaned up on exit | exists |
| `just check` / `just lint` | compiled + linted, both targets | exists, unchanged by this document |
| `just tf-fmt-check` / `just tf-validate` | static Terraform checks | exists |

There is no CI pipeline (`docs/design/index.md` already says so), and
introducing one is out of scope for this document — it defines what runs
locally, as `just` recipes, and nothing here assumes or waits on CI existing.

## Constraints

- No mocking framework, in any crate, for any of the tests this document
  proposes. `server`'s existing tests reach a real `Store::Memory`, a real
  `Router`, and a really-signed token instead, and new tests should keep
  doing that.
- No dev-dependency beyond `tower` (`util` feature) and what a handful of
  async tests already need from `tokio`. Anything heavier — `wiremock`,
  `testcontainers`, a DynamoDB fake in Rust — repeats a rejection DR-0020
  already made for the store itself: a second implementation of behaviour the
  SDK already defines, drifting from the real one.
- `just test` must keep needing no Java, no AWS credentials, and no network.
  `dynamo_tests` stays `#[ignore]`d and reached only through `just
  test-dynamo`; any future real-Cognito-backed test stays opt-in the same
  way — the same "no setup" property DR-0008 and DR-0018 already hold for
  `dev-api`/`dev-web`.
- DOM-dependent frontend behaviour is not automated by this document. It
  remains a manual check via `dev-web`/`dev-web-auth`, per workspace.md's
  existing constraint that the devcontainer has no browser and no Node.js.
- `crates/server` stays one binary target — no `lib.rs`, no `tests/`
  directory. Router-level tests live in `main.rs`'s own `#[cfg(test)] mod
  tests` — DR-0031. A second, independent reason to split would revisit that
  record; wanting `tests/`-directory ergonomics for their own sake, already
  weighed there, is not one on its own.
- **A passing `just test` is not the same signal for every dependency.** For
  `axum`/`tower`, it now is a real signal — the router-level tests exercise
  routing, extraction and status codes against the real thing. For
  `aws-sdk-dynamodb`, that signal is `just test-dynamo`, not `just test`,
  since `dynamo_tests` is `#[ignore]`d by default. For `hyper-rustls`/
  `aws-config` (the JWKS fetch) and anything in `crates/app`, no automated
  test exercises a version bump at all; both stay `dev-api-cognito`/`dev-web`
  checks, by hand, exactly as already described above.
- **Functional coverage and visual regression are different problems, and
  this document addresses neither for `crates/app` today.** Even if the DOM
  gap above were closed with `wasm-bindgen-test` and a headless browser, that
  would only reach whether an element exists and a click dispatches the right
  event — not whether it renders correctly. Layout/CSS regressions need
  separate screenshot-diffing infrastructure and maintained baselines, which
  is not proposed here either.
- **If E2E or visual-regression testing is ever pursued, it is a separate
  module, not a `crates/*` member.** Mature tooling for both (Playwright,
  Cypress, Percy, Chromatic and similar) is realistically Node.js-based;
  reimplementing that ecosystem in Rust is not a practical alternative to
  avoid it. It would sit beside the Cargo workspace with its own toolchain,
  the same shape `infra/`'s Terraform already has — not proposed or scoped by
  this document.
