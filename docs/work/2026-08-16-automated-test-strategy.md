# Define the automated test strategy
Status: in progress

## Request

Define an automated test strategy for this repository and record it as a file
under `docs/`. The resulting document is the basis for a discussion with the
user to refine its content — this is a collaborative draft, not a final
answer handed down in one pass.

Presented with four open points from the first draft, the user: found the
router-test structural choice (factor a function inside `main.rs` vs. split
into `lib.rs`) genuinely hard to judge without seeing the considerations
first, and asked for those before deciding. Confirmed the opt-in DynamoDB
integration layer should be built now rather than left as a stated target,
since it is not much additional work. Confirmed the `auth.rs` pure-logic
refactor stays deferred to a later, separate task. Confirmed introducing CI
itself is out of scope for now — a `just` recipe is enough if one is needed,
with no GitHub Actions pipeline to design.

## Interpretation

"Automated test strategy" covers the Rust workspace (`crates/shared`,
`crates/server`, `crates/app`) and, briefly, `infra/`: what is already
verified by `cargo test` and how, what gaps exist, and what closes them
without requiring AWS credentials in the common case. It does not mean
building a CI pipeline in this pass — `docs/design/index.md` already notes
one does not exist yet — only specifying what one should run once it does.

Out of scope for this pass, confirmed by the user: the `auth.rs` refactor
toward testable pure functions, and introducing a CI pipeline. ~~The opt-in
DynamoDB integration layer~~ — no longer out of scope; the user asked for it
to be built now (see Request). The router-level tests remain undecided
pending the pros/cons the user asked for.

## Plan

1. Read the existing documentation model (`docs/README.md`) and design
   documents to match the repository's own conventions rather than
   introducing a new document shape.
2. Survey the current test suite: what exists, where, in what style, and what
   it does and does not reach.
3. Draft `docs/design/testing.md` as a Design Document: current coverage,
   proposed additions with concrete reasoning, and what stays manual and why.
4. List it in `docs/design/index.md`.
5. Discuss with the user; revise the document as the discussion settles
   specific choices (router test structure, whether to build the DynamoDB
   layer now, CI scope).
   - (resolved) DynamoDB layer: build now.
   - (resolved) `auth.rs` refactor: deferred to a separate task.
   - (resolved) CI: out of scope; a `just` recipe suffices if one turns out
     to be needed.
   - (open) Router test structure: present considerations, pros and cons for
     both options; wait for the user's call before touching `main.rs`.
6. Implement the DynamoDB opt-in test layer: `#[ignore]`-gated tests in
   `store.rs` against `Store::Dynamo`, and a `just test-dynamo` recipe that
   is self-contained (starts DynamoDB Local, waits for it, tears it down)
   rather than assuming another terminal is already running it. Verify with
   `cargo test`, `cargo fmt --check`, `cargo clippy`. Update `testing.md` to
   describe it as implemented rather than proposed, and write a Decision
   Record for the choices it embeds.

## Progress

**2026-08-16.** Surveyed the workspace: `crates/server` has 38 tests, all
in-module `#[cfg(test)] mod tests`, no dev-dependencies, no mocking crate —
`testkey.rs` is a committed RSA fixture used to sign real tokens for the
Cognito verification tests instead. `crates/shared` and `crates/app` have
zero tests. Confirmed `cargo test --workspace` already compiles `crates/app`
for the host target (it reports "0 tests" successfully), which means
non-web-sys logic there is unit-testable today with no new tooling. No
`tests/` directory anywhere, no CI (`.github` does not exist), and
`backend.md`/`workspace.md` already document the two manual-only gaps this
strategy has to account for: the DynamoDB half of `Store` (DR-0020) and
anything touching the DOM in `crates/app` (no browser, no Node.js in the
devcontainer).

Drafted `docs/design/testing.md` and added it to `docs/design/index.md`.
Four points were put to the user rather than decided unilaterally: how to
make `server`'s `Router` reachable from a test, whether to build the opt-in
DynamoDB test layer now, whether the `auth.rs` pure-logic extraction is in
scope for this pass, and whether introducing CI itself belongs in this
document's scope. Answers in Request above.

**2026-08-16, continued.** Implemented the DynamoDB opt-in test layer per the
user's answer: `store::dynamo_tests` (four `#[ignore]`d tests reusing
`Store::from_environment`) and the self-contained `just test-dynamo` recipe.
Ran it end to end against a real DynamoDB Local in this environment — all
four passed, and the java process was confirmed gone afterward, including
after a normal `cargo test --workspace` run (38 passed, 4 ignored,
unaffected). `cargo fmt -p server --check` and `cargo clippy -p server
--all-targets -- -D warnings` both clean. Updated `testing.md`'s coverage
table, Structure, Interfaces and Constraints sections to describe this as
implemented rather than proposed, and wrote
[DR-0030](../decisions/DR-0030-dynamodb-store-tests-run-opt-in-and-self-contained.md)
for the choices it embeds (the `#[ignore]` mechanism, the loud-failure guard
against a silent `Memory` fallback, and deferring process cleanup to the
existing `dynamo-stop` rather than tracking a job pid by hand), added to
`docs/design/index.md`'s Decision Records table.

Added the router-level-test pros/cons the user asked for directly into
`testing.md`'s "Proposed: router-level tests" section (Option A: factor a
function inside `main.rs`; Option B: split into `lib.rs` + `tests/`), rather
than only in chat, so the comparison stays with the document it informs.

**2026-08-16, continued further.** The user's general default is Option B
(the library split, for the honesty about public API it gives tests) but
judged that reason alone insufficient to spend on the current, one-test-module
situation, and chose Option A — explicitly reversible if a second, independent
reason to split shows up. Implemented accordingly: factored `fn router(state:
AppState) -> Router` out of `main()` in `crates/server/src/main.rs`, added a
`#[cfg(test)] mod tests` to the same file with seven tests against the real
`Router` via `tower::ServiceExt::oneshot` (`tower` added as a dev-dependency,
`util` feature only, declared in the root `Cargo.toml`'s
`[workspace.dependencies]` alongside a comment matching this project's
existing `hyper-util`/`hyper-rustls` style for "already in `Cargo.lock`,
declaring it adds an edge, not a package"). The seven: a full create-then-list
round trip; a malformed JSON body answering axum's own `400`; an unmapped
method answering `405`; an edge naming nobody answering `401` end to end
(not just at `mock_caller()` in isolation, which `identity.rs`'s own test
already covered); the same for a missing token under `Auth::Cognito`; a
verified Cognito token reaching the handler; and `/health` reachable with no
auth headers at all.

Wrote [DR-0031](../decisions/DR-0031-router-level-tests-live-inside-main-rs.md)
for this decision, recording the user's stated general preference for the
library split and the reasoning for not spending it here, so a future reader
does not read the chosen shape as disagreement with that preference. Updated
`docs/design/index.md`'s Decision Records table, `testing.md` (moved the
router-level-tests section from "Proposed" into "Today," updated the test
count, replaced the inline pros/cons with a citation to DR-0031), and
`backend.md`/`workspace.md` (the `src/main.rs`, `src/store.rs` and
`src/testkey.rs` Structure-table rows, the stale "`cargo test` reaches only
the in-memory half... which nothing does automatically" constraint, and a new
`test-dynamo` row in the Tasks table) to reflect both layers as implemented.

**2026-08-16, continued once more.** Asked to judge test quality against a
different bar: whether a passing test suite is sufficient signal to release
after a dependency bump (leptos, axum, and similar were named), and, relatedly,
whether this project's tests could stand up to SBOM-style dependency-risk
scrutiny. Confirmed there is no dependency-audit or SBOM tooling in the repo
at all (no `cargo-audit`/`cargo-deny`, no Dependabot/Renovate config, no CI).
Answer given: no, not uniformly — `axum`/`tower` now get a real signal from
`just test` (the router-level tests), `aws-sdk-dynamodb` only gets one from
`just test-dynamo` specifically, and `hyper-rustls`/`aws-config`'s JWKS fetch
and all of `crates/app` get none at all. Separately, corrected an
over-generalisation in that answer: even a future DOM/event-level E2E layer
for `crates/app` would only reach functional presence and event dispatch, not
visual correctness — layout/CSS regressions are a distinct problem needing
screenshot-diffing infrastructure, which is a different investment than
closing the DOM-testing gap already named in `testing.md`. The user confirmed
this framing was exactly the missing piece and asked for a short addendum
only — no dependency-monitoring or CI work, which stays explicitly deferred.
Added two `Constraints` bullets to `testing.md` capturing both points; no
code changes this round.

**2026-08-16, closing.** Discussed whether Claude in Chrome (Claude Code's
browser-extension integration, confirmed real via web search — see the chat
turn) shortcuts the `crates/app` DOM-testing gap. Conclusion: it is a genuine
fit for strengthening the existing manual `dev-web` check — it runs on the
host's real Chrome, so it needs no devcontainer change — but it is an
interactive, agent-in-the-loop session, not a deterministic, CI-gateable test;
not adopted or scoped into this document. Separately, the user concluded that
E2E and visual-regression tooling realistically cannot avoid Node.js (mature
tools — Playwright, Cypress, Percy, Chromatic — are all Node.js-based;
reimplementing that in Rust is not practical), so either, if ever pursued,
would be a separate module beside the Cargo workspace with its own toolchain,
the same shape `infra/`'s Terraform already has. Added one `Constraints`
bullet to `testing.md` recording this. User confirmed this closes the
session ("それで区切りとします").

## Verification

`cargo test --workspace` run as a baseline before drafting (38 passed, 0
failed), again after the DynamoDB layer landed (38 passed, 4 ignored), and
again after the router-level tests landed (45 passed, 4 ignored — all seven
new tests included). `just test-dynamo` run twice, before and after the
`main.rs` refactor: 4 passed both times, DynamoDB Local confirmed stopped
afterward by scanning `/proc` for a `java` process. `just fmt-check`, `just
check` and `just lint` (both targets, `-D warnings`) all clean on the full
workspace after every round of changes, most recently after the router-level
tests landed. The final round (this entry) is documentation only, so no
further `cargo`/`just` verification applies to it.

## Retirement

- [x] Design Documents updated — `testing.md`, `backend.md` and
      `workspace.md` all reflect the DynamoDB and router-level test layers as
      implemented. `testing.md`'s Purpose section states plainly that it
      awaits the confirmation this checklist item is asking for; that
      confirmation is the one thing this Work Log cannot itself supply — see
      the chat turn this log accompanies.
- [x] Decision Records written — DR-0030 (DynamoDB opt-in layer) and DR-0031
      (router-level tests stay inside `main.rs`, recording the user's general
      preference for the library split and why it was not spent here).
- [x] Non-obvious knowledge preserved — both DRs above; nothing surfaced
      during this work that lacks a home in either a DR or in `testing.md`
      itself.
- [ ] No durable document depends on this log — true once the design-document
      confirmation above lands; nothing currently cites this log by name.
