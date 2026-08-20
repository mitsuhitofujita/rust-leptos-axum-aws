# Home's loading state briefly flashes the signed-out heading

Status: complete
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

**Superseded (2026-08-20), by user's live-browser check.** Step 5's compiler-
only verification was not enough: the user tested in a real browser and the
flash still occurred, specifically when signed out of the app but still
signed in to Google. That is exactly the one case steps 1–3 deliberately left
alone — a `code` returning from the hosted UI, settled through `Loading` +
`spawn_local`'s async exchange — on the Interpretation's assumption that a
real network round trip is long enough for `Loading` to be a legitimate,
visible wait rather than a flash. That assumption is what was wrong: with an
already-authenticated Google session, the redirect back and the token
exchange both complete fast enough to reproduce the same mount-then-tear-down
of `SignedOutIntro`'s entrance animation, just narrowed down to this one
path instead of every ordinary load.

6. In `crates/app/src/home.rs`, drop `<SignedOutIntro/>` from `HomePage`'s
   `AuthState::Loading` arm entirely, rendering just `<SiteHeader/>` and the
   `"Checking your session…"` status line — the same minimal composition
   `RequireAuth`'s own `Access::Pending` view already uses in `app.rs`,
   which does not have this bug. No heading animation ever starts during
   `Loading`, so it does not matter anymore how quickly the exchange settles.
7. Re-verify with `just check` / `just lint`.

**Superseded (2026-08-20), by a second round of user browser testing.** Step
6 removed the motion but not the flash: the user reported the animation was
gone, but the screen still flashed for an instant. Steps 6–7's implicit
premise — that the flash *was* the entrance animation — was incomplete. With
no motion involved, what remains is a plain, instantaneous content swap: a
short single line ("Checking your session…") replaced without transition by
`SignedInHome`'s much taller composition (heading, account row, dashboard
card). Removing the animation stopped the wrong heading from visibly
sliding/fading in, but the abrupt height and content change between the two
DOM subtrees is still perceptible as a flash on its own, independent of any
CSS `animation`. The user's own diagnosis, offered unprompted: keep a
title-sized presence on screen throughout `Loading`, rather than a small
caption that then grows into a full heading — smaller shape change, smaller
flash.

8. In `crates/app/src/home.rs`, change `HomePage`'s `AuthState::Loading` arm
   to render the "Checking your session…" copy as an `<h1>` occupying the
   same position and size `.page-heading`'s title takes (`style/main.css`
   gets a `.loading-title` rule carrying `.page-heading`'s `margin-top`,
   including its short-viewport contraction, but deliberately not its `enter`
   animation), replacing the small `<p class="status">` used since step 6.
   The wording stays exactly what was already shown; only its visual weight
   and position change, so the eventual real heading replaces text roughly
   in place rather than growing a small line into a large one.
9. Re-verify with `just check` / `just lint`.

**Superseded (2026-08-20), by user feedback on step 8's result.** The `<h1>`
placeholder was "too prominent" (目立ちすぎ) for a state meant to last a
moment at most — display-size, weight-800 text is a lot of visual weight to
commit to for text that is likely to be replaced almost immediately. The
user asked for the same copy at the eyebrow's weight and position instead of
the title's.

10. In `crates/app/src/home.rs`, render the `Loading` copy as
    `<p class="eyebrow loading-eyebrow">` rather than `<h1 class=
    "loading-title">` — the small, uppercase, tracked-out treatment every
    other screen's eyebrow already uses, with no accompanying title.
    `style/main.css` replaces `.loading-title` with `p.loading-eyebrow`,
    positioned beside `.eyebrow`'s own rule: `margin-top: var(--space-lg)`
    (`var(--space-md)` under the existing `max-height: 720px` contraction),
    using an element+class selector so it wins over the plain `.eyebrow`
    rule's `margin: 0 0 var(--space-md)` regardless of source order, rather
    than depending on declaration order to break the specificity tie.
11. Re-verify with `just check` / `just lint`.

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

The user then checked in a real browser: signed out of the app but still
signed in to Google, the flash still appeared. This is the `code`-exchange
path steps 1–3 left alone (see Plan's superseded note above) — with an
already-live Google session, the round trip through the hosted UI and the
token exchange both complete fast enough to reproduce the same
mount-then-replace of `SignedOutIntro`'s entrance animation that the
ordinary-load case had.

Implemented Plan step 6: `crates/app/src/home.rs`'s `AuthState::Loading` arm
no longer renders `<SignedOutIntro/>` or the `.auth-panel` wrapper around the
status line — just `<SiteHeader/>` and `<p class="status">"Checking your
session…"</p>`, mirroring `RequireAuth`'s `Access::Pending` view in `app.rs`,
which never had this problem because it never mounted an animated heading in
the first place. `.page-heading`'s `enter` animation (`style/main.css`) now
never starts during `Loading` at all, so the flash cannot occur regardless of
how fast the exchange resolves. `just check` / `just lint` stay clean (step
7). Not yet re-confirmed in a live browser.

The user re-checked: the animation was confirmed gone, but a brief, non-
animated flash remained — the plain content swap described in the Plan's
second superseded note above. The user's own suggestion was adopted rather
than the delay-before-show alternative that had been offered: keep a
title-sized presence on screen throughout `Loading` so the eventual swap is
smaller.

Implemented Plan step 8: `home.rs`'s `Loading` arm now renders
`<h1 class="loading-title">"Checking your session…"</h1>` in place of the
`<p class="status">` step 6 used — same wording, now at the same size and
position `.page-heading`'s own `<h1>` occupies. `style/main.css` gained
`.loading-title { margin: var(--space-lg) 0 0; }` beside `.page-heading`
(plus its `max-height: 720px` contraction to `--space-md`, mirroring
`.page-heading`'s own), deliberately without `.page-heading`'s `animation:
enter` rule — the bare `h1` selector already supplies the display-size
typography, so no other new CSS was needed. `.status` remains defined and
used elsewhere (`RequireAuth`'s `Access::Pending` in `app.rs`); nothing else
was touched. `just check` / `just lint` stay clean (step 9). Not yet
re-confirmed in a live browser.

The user called the `<h1>` placeholder too prominent (目立ちすぎ) for
something meant to be on screen for a moment at most, and asked for the
eyebrow's weight and position instead of the title's.

Implemented Plan step 10: `home.rs`'s `Loading` arm now renders
`<p class="eyebrow loading-eyebrow">"Checking your session…"</p>`, dropping
`.loading-title`/`<h1>` entirely. `style/main.css` replaces the
`.loading-title` rule with `p.loading-eyebrow { margin-top: var(--space-lg);
}`, placed directly after `.eyebrow`'s own block, and uses the element+class
selector specifically so its `margin-top` wins over `.eyebrow`'s `margin: 0 0
var(--space-md)` shorthand by specificity rather than by depending on which
rule happens to come last in the file. The `max-height: 720px` contraction
was updated to match (`p.loading-eyebrow { margin-top: var(--space-md); }`).
No accompanying title is rendered — the eyebrow line is the entire
composition below `SiteHeader` while `Loading`. `just check` / `just lint`
stay clean (step 11). Not yet re-confirmed in a live browser.

**Possible Decision Record.** Two findings here look durable enough to
outlive this Work Log:

1. `App`'s auth-settling code always took the `Loading` → `spawn_local` path
   regardless of whether the answer was actually knowable without the
   network — seeding a signal synchronously wherever the async wrapper's own
   logic can already answer without awaiting anything is the general fix.
2. The one that actually closed the reported bug: a transient/provisional
   composition (`Loading`, reached for a window whose length is not
   controlled by this code — it ends whenever a network call the app does
   not control happens to finish) should not carry an entrance animation on
   content the settled state does not share, no matter how long that window
   is expected to last. `RequireAuth`'s `Access::Pending` view already
   followed this without it being written down anywhere; `HomePage`'s
   `Loading` arm did not, and guessing that "a real network round trip is
   long enough not to flash" (this log's original Interpretation) turned out
   to be exactly the kind of unverified assumption about external timing the
   work-log skill warns against — it was wrong the first time it met a real
   browser.

Whether either is worth a Decision Record is flagged to the user rather than
decided here.

## Verification

- `just check` (`cargo check --workspace` and `cargo check -p app --target
  wasm32-unknown-unknown`): clean.
- `just lint` (`cargo clippy --workspace --all-targets -- -D warnings` and
  the same for `-p app --target wasm32-unknown-unknown`): clean, no warnings.
- `cargo test --workspace`: 64 passed, 10 ignored (unchanged, all in
  `server`); `app` still compiles and runs with 0 tests, as testing.md
  describes.
- No live-browser check from this side: `crates/app` has no automated tests
  and this devcontainer has no browser (testing.md, workspace.md), so
  compiler checks and a by-hand trace of all five `AuthState` outcomes were
  as far as verification here could go on its own.
- The user tested each round of changes in a real browser. The first round
  (steps 1–3) and the second (step 6) each turned out incomplete — see the
  two superseded Plan notes and the corresponding Progress entries. The
  third round (step 10, the eyebrow treatment) was confirmed by the user to
  have resolved the flash.

## Retirement

- [x] Design Documents updated — `frontend.md`'s Authentication paragraph and
      a new "Home, settling" subsection in `page-layouts.md`.
- [x] Decision Records written — DR-0036 (synchronous `AuthState` settling),
      DR-0037 (a still-settling composition's eyebrow-only treatment).
- [x] Non-obvious knowledge preserved — the three rejected `Loading` visual
      treatments and why each still flashed are in DR-0037; the rejected
      delay-before-show alternative is in both records.
- [x] No durable document depends on this log — checked by grep.
