# Home's loading state briefly flashes the signed-out heading

Status: in progress
Started: 2026-08-20
Branch: main

## Request

When sign-in succeeds and the home screen goes on to show the account's
information, the signed-out heading's entrance animation — "Make every action
count." — briefly appears first, for an instant, before the correct content
shows. Fix it.

## Interpretation

**What's being asked.** Stop the signed-out intro's animated heading from
flashing on screen for a returning, already-authenticated visitor before the
signed-in home replaces it.

**Root cause.** `App` (`crates/app/src/app.rs`) always seeds `auth_state` with
`AuthState::Loading` at mount and resolves it asynchronously, inside
`spawn_local`, by awaiting `auth::complete_sign_in()`. `HomePage`
(`crates/app/src/home.rs`) renders `<SignedOutIntro/>` — the CSS-animated
"Make every action count." heading (`.page-heading`'s `enter` keyframe,
`style/main.css`) — for that `Loading` state, alongside "Checking your
session…".

But `complete_sign_in`'s own `settle()` only needs the network for one of its
three cases: a `code` in the query string, returning from the Cognito hosted
UI, which is genuinely exchanged over HTTP. The other two — an ordinary load
with or without a stored session, and a `?error=` the hosted UI sent back —
are resolved synchronously today, already, just from `location.search()` and
`sessionStorage`; they are only wrapped in an `async fn` because they share
`settle()`'s signature with the case that isn't. Because `App` always takes the
`Loading` → `spawn_local` path regardless, the ordinary-load case — which is
what a returning, already-signed-in visitor hits on every normal open — still
pays for one render pass as `Loading` before the signal is overwritten a
moment later. That render pass is enough to mount `SignedOutIntro`'s
`page-heading`, start its 500ms entrance animation, and then tear it down
again when `SignedInHome`'s own heading replaces it — the flash reported.

**Scope.** Fix is confined to `crates/app/src/auth.rs` (expose the
already-synchronous part of `settle()` as its own function, callable before
any `.await`) and `crates/app/src/app.rs` (seed `auth_state`'s initial value
from it directly, falling back to `Loading` + `spawn_local` only for the one
case — a `code` present — that genuinely can't be resolved synchronously).
Nothing changes in `home.rs`, the CSS, or the design docs: `Loading` and its
shared `SignedOutIntro` rendering stay correct and necessary for the
code-exchange case, which is a real network round trip long enough that
"Checking your session…" is a legitimate loading state, not a flash — DR-0011
already describes `Loading` as "the window in which `complete_sign_in` is
exchanging an authorization code," which is narrower than what the code
currently does; this brings the implementation in line with that description
rather than changing it.

**Out of scope.** The same fresh-mount re-entrance-animation behavior for the
code-exchange case itself (Google → back to `/`, `Loading` → `SignedInHome`)
is not touched. That transition is not what was reported, and unlike the
ordinary-load case it spans a real network request, so there is no
instantaneous mis-paint to eliminate — only fresh content replacing a
heading that was genuinely shown to explain a genuine wait.

**Assumption, not depended on.** The flash is consistent with
`wasm_bindgen_futures::spawn_local` deferring its future's first poll past the
initial synchronous render (a microtask, not the same call stack as `App()`'s
body), so the first paint already carries `Loading` before the signal is
overwritten. This is not verified against the dependency's source and isn't
being verified here, because the fix does not rely on it either way: moving
the synchronous cases out of `spawn_local` entirely means the very first
value handed to `RwSignal::new` is already correct for those cases, regardless
of how `spawn_local` happens to schedule its poll.

## Plan

1. In `crates/app/src/auth.rs`, factor `settle()`'s synchronous branches (the
   `error` query param and the plain `restore_session()` path) into a
   `settle_without_code(query: &UrlSearchParams) -> Result<Option<AuthState>, String>`,
   returning `Ok(None)` only when a `code` is present. `settle()` calls it
   first and only continues into the exchange when it gets `None`.
2. Add `pub fn initial_state() -> Option<AuthState>` to `auth.rs`: checks
   `is_configured()`, parses the current query once, and delegates to
   `settle_without_code`. Returns `None` exactly when `complete_sign_in`'s
   async exchange is still needed.
3. In `crates/app/src/app.rs`'s `App()`, seed `auth_state` from
   `auth::initial_state()`, falling back to `AuthState::Loading` only on
   `None`, and only `spawn_local` the `complete_sign_in().await` call in that
   `None` case. The three other outcomes never render `Loading` at all — the
   first render already carries the settled state.
4. Leave `home.rs`, `style/main.css`, and the design docs untouched.
5. Verify with `just check` / `just lint` (`cargo check`/`clippy -p app
   --target wasm32-unknown-unknown`) and by tracing all four cases
   (no code/no error with a session, no code/no error without one, `error`
   present, `code` present) against both the old and new code paths to confirm
   the resulting `AuthState` is identical in every case except that three of
   the four now skip `Loading`. `crates/app` has no automated tests today and
   this workspace's devcontainer has no browser (testing.md, workspace.md), so
   there is no automated or live-browser check available beyond this and the
   compiler.

## Progress

### 2026-08-20

Implemented the plan as written, steps 1–3.

- `crates/app/src/auth.rs`: factored `settle()`'s two synchronous branches
  (`error` in the query, and the plain no-`code` path) into
  `settle_without_code(query: &UrlSearchParams) -> Result<Option<AuthState>, String>`,
  returning `Ok(None)` only when a `code` is present. Added
  `pub fn initial_state() -> Option<AuthState>`, which checks
  `is_configured()` and then delegates to `settle_without_code` against the
  current query, returning `None` exactly when the caller must fall back to
  `complete_sign_in`'s async exchange. `settle()` itself now calls
  `settle_without_code` first and only builds the exchange request when it
  gets `None`, sharing the query parse (`current_query()`) with
  `initial_state`.
- `crates/app/src/app.rs`: `App()` now seeds `auth_state` from
  `auth::initial_state()` — falling back to `AuthState::Loading` only when it
  is `None` — and only spawns the `complete_sign_in().await` task in that
  `None` case. `Disabled`, `SignedOut`, `SignedIn` (an ordinary load with a
  stored session) and the hosted-UI-`error` case now reach the very first
  render already settled; the `Loading` composition is reachable only for a
  genuine `code` exchange, matching DR-0011's existing description of what
  `Loading` is for more literally than the prior code did.
- `home.rs`, `style/main.css`, and the design docs were left untouched, as
  planned — `Loading`'s `SignedOutIntro` rendering is still correct for the
  one case that still uses it.

Traced all five outcomes (`Disabled`; no code/no error with a session; no
code/no error without one; `error` present; `code` present) against both the
pre-change and post-change code by hand — the resulting `AuthState` is
identical in every case, and four of the five now skip `Loading` entirely.
The `error` and `Disabled` cases turned out to have the same flash under the
old code (their side effects — `forget_transient`/`clean_url` for `error` —
are unaffected, just now run synchronously at mount instead of inside the
microtask `spawn_local` schedules); fixing them was incidental to following
the plan, not a scope change, since they fall under the same
`settle_without_code`/`is_configured()` synchronous path step 2 already
covered.

**Possible Decision Record.** The fix generalizes to a pattern this project
had not written down: `App`'s auth-settling code always took the
`Loading` → `spawn_local` path regardless of whether the answer was actually
knowable without the network, which is what let a CSS entrance animation on
`.page-heading` flash between two fresh mounts of visually different content
for an outcome that was, in every case but one, already decided by the time
`App()`'s body ran. Whether this is worth a Decision Record — the general
rule "seed a signal synchronously wherever the async wrapper's own logic can
already answer without awaiting anything" — is flagged to the user rather
than decided here.

## Verification

- `just check` (`cargo check --workspace` and `cargo check -p app --target
  wasm32-unknown-unknown`): clean.
- `just lint` (`cargo clippy --workspace --all-targets -- -D warnings` and
  the same for `-p app --target wasm32-unknown-unknown`): clean, no warnings.
- `cargo test --workspace`: 64 passed, 10 ignored (unchanged, all in
  `server`); `app` still compiles and runs with 0 tests, as testing.md
  describes.
- No live-browser check: `crates/app` has no automated tests and this
  devcontainer has no browser (testing.md, workspace.md), so the flash itself
  was not observed disappearing in a running app — verification is the
  by-hand trace of all five `AuthState` outcomes in the Progress entry above,
  which shows the new code produces the same final state as the old code in
  every case, while four of five no longer pass through `Loading` at all.

## Retirement

- [ ] Design Documents updated
- [ ] Decision Records written (DR-____)
- [ ] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
