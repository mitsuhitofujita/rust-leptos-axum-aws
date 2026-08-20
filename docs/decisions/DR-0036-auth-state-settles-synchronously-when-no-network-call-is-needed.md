# DR-0036: `AuthState` settles synchronously whenever `complete_sign_in`'s own logic needs no network call

Status: accepted
Date: 2026-08-20

## Context

`App` always seeded `auth_state` with `AuthState::Loading` at mount and
resolved it asynchronously, inside `spawn_local`, by awaiting
`auth::complete_sign_in()`. `HomePage` rendered the signed-out home's
animated heading for that `Loading` state.

But `complete_sign_in`'s own `settle()` genuinely needs the network for only
one of its three cases: a `code` in the query string, returning from the
Cognito hosted UI. The other two — an ordinary load with or without a stored
session, and a `?error=` the hosted UI sent back — were already resolved
synchronously, just from `location.search()` and `sessionStorage`; they were
only wrapped in an `async fn` because they shared `settle()`'s signature with
the one case that isn't. Because `App` took the `Loading` → `spawn_local`
path regardless of which case applied, an already-authenticated returning
visitor's home still passed through one `Loading` render before the real
state overwrote it — visible as a flash of the signed-out heading's entrance
animation.

## Decision

`crates/app/src/auth.rs` exposes the synchronous branches of `settle()` as
`settle_without_code(query: &UrlSearchParams) -> Result<Option<AuthState>, String>`,
returning `Ok(None)` only when a `code` is present, and a new
`pub fn initial_state() -> Option<AuthState>` built on it, resolving
everything except a pending code exchange before any `.await`. `App()` seeds
`auth_state` from `initial_state()` directly and falls back to
`AuthState::Loading` plus `spawn_local`'s `complete_sign_in().await` only
when `initial_state()` returns `None` — exactly the code-exchange case.
`Loading` is therefore reached only for that one genuinely asynchronous case,
matching DR-0011's existing description of it — "the window in which
`complete_sign_in` is exchanging an authorization code" — more literally than
the code previously did.

## Alternatives

**Keep resolving everything through `spawn_local`, but delay showing any
`Loading`-specific UI until a short threshold (e.g. 100–150ms) has passed**,
so a fast resolution never paints an intermediate frame. Rejected: it still
renders `Loading`'s composition into the signal for every load and only
optionally defers what gets drawn from it; it adds a timer and a signal to
reason about even though four of the five outcomes need no asynchrony at all,
and it tries to outrun the scheduling race rather than removing it.

**Leave `settle()` as one async function and only change what `Loading`
renders.** This is the direction later taken for the one case that remains
genuinely asynchronous (DR-0037), but it does not by itself address the
render pass every ordinary load was paying for. Rejected as the sole fix:
`HomePage`'s `Loading` arm would still mount and unmount once per ordinary
load, leaving the code not matching DR-0011's own description of what
`Loading` is for.

## Consequences

Easy: a screen or a future auth-consuming component can treat
`AuthState::Loading` as meaning "an authorization code is actually being
exchanged," not "the state hasn't been checked yet" — which is what DR-0011
already claimed. `auth.rs` gained `current_query()`, shared between
`initial_state` and `settle` so the query string is parsed once per caller.

Hard, and accepted: `auth.rs` now has two entry points into the same
decision — `initial_state`, called synchronously at mount, and `settle`,
called from `complete_sign_in` — that must stay in agreement.
`settle_without_code` is what keeps them from drifting apart, but a future
change to one of the three branches has to remember the other caller shares
it. Reversing this means moving `initial_state`'s logic back inside
`complete_sign_in` and always spawning it from `App()`.
