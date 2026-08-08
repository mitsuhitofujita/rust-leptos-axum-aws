# Route guards for authenticated screens

Status: in progress
Started: 2026-08-08
Branch: main

## Request

Authentication is now implemented. Design how route guarding should work in this
application and propose it before building anything.

Separately, advise where a Cognito session is best held in a client, given that
Cognito is what supplies the identity.

### Clarifications

The return destination is not preserved. A visitor bounced off a protected route
signs in and lands on home; the dashboard card is one tap from there. No stored
intended path, no extra build surface.

The guard owns the redirect on an unauthorized response. A 401 demotes the
session and nothing more; the guard is what moves the visitor. The dashboard's
inline "sign in and try again" message becomes unreachable and is removed.

The unit of work is the route guard alone. The two other findings raised while
proposing — a Content Security Policy for the delivered bundle, and the
disagreement between Page Layouts and the implemented signed-in home over the
account email — are not part of this work.

## Interpretation

**What is being asked.** A guard that decides which screens an unauthenticated
visitor may reach, applied to the router in `crates/app`, plus the durable
records that explain it. The proposal has been reviewed and its open questions
answered, so this log covers building it as well as designing it.

**What the storage question resolved to.** The advice was that the current
arrangement — `sessionStorage`, holding an access token with no refresh token,
per DR-0010 — is already the right choice for a bundle served as static files
with no session-holding server of its own. No change follows from it. What would
genuinely improve the position is a CSP on the delivered bundle, which the
clarification places out of scope. This log therefore produces no storage change;
the reasoning belongs in the route-guard Decision Record only where it explains
why the guard cannot be a security boundary.

**Out of scope.**

- Any change to `auth.rs`'s storage keys, token lifetime, or refresh behaviour.
- The CSP work. Noted here because it was discovered during this request and has
  a concrete obstacle attached: `dist/index.html` carries one inline
  `<script type="module">`, whose hash changes every build, so `script-src
  'self'` cannot be adopted without generating the header per deploy.
- The account email. `docs/design/page-layouts.md` requires it in the signed-in
  home's account strip; `crates/app/src/home.rs` does not render it, and
  `auth.rs` stores it while stating that nothing displays it. This is a real
  disagreement between a Design Document and the code, and it needs its own unit
  of work.
- Server-side identity. Once action types and actions exist, the API must scope
  data by the authorizer-verified `sub` rather than anything the client sends.
  That is a separate decision, and it interacts with local development, which has
  no authorizer in front of it.
- Building the screens the guard will eventually cover. Only `/dashboard` exists
  today.

**Assumptions.**

- The guard is a user-experience mechanism, not an authorization boundary. API
  Gateway's JWT authorizer remains the only enforcement point (DR-0010), and the
  guard's purpose is to keep an unauthenticated visitor off a screen that would
  render a 401 rather than to protect anything.
- `/` stays a single route with two compositions. `docs/design/page-layouts.md`
  defines signed-out and signed-in home as two states of one screen, so it is not
  guarded, and a signed-in visitor arriving at `/` is not redirected away from it.
- `AuthState::Disabled` must pass the guard. It means a build with no Cognito
  configuration, and blocking it would put `just dev-web` behind a sign-in that
  does not exist — contradicting DR-0008, where an unset variable means something
  workable.
- `AuthState::Loading` must hold rather than reject. It covers the window in
  which `complete_sign_in` is exchanging an authorization code, so rejecting it
  would eject the visitor who is in the middle of returning from the hosted UI.
- Guarded routes are declared in the router, so that reading the route table is
  enough to see which screens require a session.

## Plan

1. Add a `RequireAuth` component that maps the five `AuthState` values onto three
   outcomes: render for `SignedIn` and `Disabled`, hold with a pending view for
   `Loading`, redirect to `/` for `SignedOut` and `Error`. Its `children` prop is
   `ChildrenFn`, not `Children`, because the auth state changing re-renders it.
   The redirect runs in an `Effect` with `NavigateOptions { replace: true, .. }`,
   so the back button does not bounce between the protected route and home.
2. Apply it to `/dashboard` in `app.rs`'s route table.
3. Move the 401 demotion out of `dashboard.rs` into one shared place, keeping the
   existing rule that only a signed-in state transitions. Delete the now
   unreachable `ApiError::Unauthorized` branch from the dashboard's view.
4. Verify: `just fmt`, `just lint`, `just check`, then by hand under `just
   dev-web` (unconfigured, so `/dashboard` must render) and `just dev-web-auth`
   (signed out, signed in, deep link to `/dashboard` while signed out, and the
   return trip from the hosted UI landing on `/dashboard` rather than being
   ejected mid-exchange).
5. Draft DR-0011 covering the guard: its status as user experience rather than
   security, the mapping of the five states, why `leptos_router`'s
   `ProtectedRoute` was not used, why the return destination is not preserved,
   and why the 401 no longer redirects on its own.
6. Draft the updates to `docs/design/frontend.md` and
   `docs/design/page-layouts.md` and have them confirmed before this log retires.

## Progress

### 2026-08-08

Log opened. The proposal that preceded it established the design and its three
open questions; the answers are recorded under Clarifications above.

Two findings from reading the durable layer and the crate, both of which shaped
the plan:

`leptos_router` 0.8.15 does ship `ProtectedRoute`, and its `condition: Fn() ->
Option<bool>` maps onto the loading state exactly. It was still rejected:
`components.rs:437` renders `<Redirect path=.../>` with no `NavigateOptions`, and
`replace` defaults to `false`, so every rejection pushes a history entry and the
back button bounces. A hand-written wrapper is a few lines and controls that.

The 401 handling and the guard overlap. `dashboard.rs:37-44` already demotes the
session to `SignedOut` on a 401; with a guard in place that demotion is what
moves the visitor, which makes the inline message at `dashboard.rs:78-83`
unreachable. Both mechanisms staying would mean two answers to the same event.

Two questions were put to the requester and answered. The dashboard's resource
drops its read of the auth signal and states the guard as a precondition in its
comment, rather than keeping a redundant read that no longer means what its
comment claims. `RequireAuth` and the shared demotion live in `app.rs`, beside
the `Auth` context they act on, which keeps `auth.rs` free of Leptos.

Steps 1 to 3 implemented as planned. A detail the plan did not anticipate: the
guard needs a three-valued enum behind a `Memo` rather than a boolean, because
`children()` rebuilt on every auth-state write would remount the guarded screen
and refetch its resource. The memo only notifies when the outcome changes.

One comment elsewhere went stale and was corrected: `app.rs`'s note on
`complete_sign_in` said the exchange had to finish "before ... the dashboard's
resource waits on" it, which stopped being true when the resource was decoupled.
It now names `RequireAuth` as what holds a guarded screen back.

DR-0011 written. It carries the guard's status as experience rather than
enforcement, the five-to-three mapping and why `Loading` holds and `Disabled`
passes, the rejection of `leptos_router`'s `ProtectedRoute` over its `replace`
default, why no destination is preserved, and the coupling the decoupled
resource introduced in exchange.

Design Document drafts written and awaiting confirmation: `frontend.md`
(Structure file table, Routing, Authentication, Constraints) and
`page-layouts.md` (one addition to Navigation). `index.md` gained the DR-0011
row.

While updating `frontend.md`, three statements were found to have gone stale
when the dashboard replaced the greeting screen, and were corrected in passing
because they sit in the sections this work rewrote: the file table listed
neither `home.rs`, `dashboard.rs` nor `icons.rs`; Interfaces named `GET
/api/greeting` → `shared::Greeting`; and a constraint described a 401 rendering
"where the greeting belongs". `docs/design/index.md` carries the same stale
description of the backend — it still names `GET /api/greeting`, where
`crates/server/src/main.rs:23` now routes `/api/dashboard`. That paragraph was
left alone: it is not a section this work touches, and correcting it belongs to
whoever next opens the backend question.

## Verification

`just fmt-check`, `just lint` and `just check` all pass; `just lint` covers both
the host target and `wasm32-unknown-unknown` with `-D warnings`. `trunk build`
produces a bundle.

The five browser checks in the plan have **not** been run. No browser is
available in this environment, so they are the requester's to perform:

1. `just dev-web` (unconfigured, `Disabled`) — `/dashboard` must render. This is
   the regression the guard could most easily cause.
2. `just dev-web-auth`, signed out — entering `/dashboard` must settle on `/`,
   and the back button must leave the site rather than return to `/dashboard`.
3. `just dev-web-auth` — sign in from `/`, then reach the dashboard by its card.
4. Signed in, reload on `/dashboard` — a brief "Checking your session…", then
   the dashboard, with no ejection during the `Loading` window.
5. Sign out from home, then navigate back toward `/dashboard` — the guard
   returns the visitor to `/`.

The 401 path cannot be exercised at all locally: the development API validates
nothing, so no 401 is producible under `trunk serve`. It is reasoned about in
DR-0011 rather than observed, and would first be seen against a deployed bundle.

## Retirement

- [ ] Design Documents updated — drafted, awaiting confirmation
- [x] Decision Records written (DR-0011)
- [x] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
