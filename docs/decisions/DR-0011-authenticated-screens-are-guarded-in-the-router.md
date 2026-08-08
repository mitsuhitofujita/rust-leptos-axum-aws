# DR-0011: Authenticated screens are guarded in the router, and the guard is experience rather than enforcement

Status: accepted
Date: 2026-08-08

## Context

DR-0010 gave the SPA a session but nothing that used it to decide where a
visitor may go. `/dashboard` was reachable while signed out: the screen mounted,
called `/api/dashboard`, and — against a deployed API, where API Gateway's JWT
authorizer sits in front of `/api/{proxy+}` — received a 401, which the screen
rendered as a message telling the visitor to sign in somewhere else. The visitor
was left on a broken screen holding instructions.

Page Layouts describes five more application screens still to be built. Every
one of them would inherit that failure, so the answer had to belong to the
router rather than to any screen.

Three things shaped the answer.

The security boundary was already settled and is not here. API Gateway's
authorizer is the only thing enforcing anything (DR-0010). Whatever the browser
decides about routes is advice to itself, and a visitor determined to reach a
guarded path can do so with a debugger.

The auth state has five values, not two. `AuthState` distinguishes `Loading`,
`Disabled`, `SignedOut`, `SignedIn` and `Error`, and two of those are not
"signed out" in any sense a guard should act on. `Loading` covers the window in
which `complete_sign_in` is exchanging an authorization code — the state of a
visitor who is in the middle of coming back. `Disabled` is a build with no
Cognito configuration, which DR-0008 requires to be a working state rather than
a broken one.

And the redirect URI is computed, not configured. DR-0010 fixed it at
`window.location.origin` plus a trailing slash, precisely so it cannot drift
from what the app client registers. A round trip through the hosted UI therefore
always lands on `/`, and cannot carry a destination back.

## Decision

A `RequireAuth` component in `crates/app/src/app.rs`, wrapped around the view of
every route that needs a session, starting with `/dashboard`.

- **The guard is a matter of experience, not of authorization.** It exists so an
  unauthenticated visitor never lands on a screen that can only fail. It is not
  a reason to relax anything on the server, and any future check of who owns
  what belongs behind the API.

- **Five states map onto three outcomes.** `SignedIn` and `Disabled` render the
  screen. `Loading` holds, showing the same "Checking your session…" copy the
  home screen uses. `SignedOut` and `Error` redirect to `/`. `Disabled` passing
  is what keeps `just dev-web` able to reach a guarded screen with no sign-in
  configured; `Loading` holding is what keeps a returning visitor from being
  ejected mid-exchange.

- **The mapping goes through a `Memo`, not the raw signal.** A memo over a
  three-valued enum only notifies when the outcome changes, so an auth-state
  write that leaves the answer alone does not rebuild the screen behind the
  guard — and rebuilding a screen rebuilds its resources and refetches.

- **The redirect replaces rather than pushes.** `NavigateOptions { replace: true
  }`, so the back button leaves the site instead of returning to the guarded
  path to be bounced forward again.

- **`/` is not guarded, in either direction.** Page Layouts defines signed-out
  and signed-in home as two states of one screen, so authentication changes what
  `/` renders and never where the visitor is. A signed-in visitor arriving at
  `/` stays there.

- **The intended destination is not preserved.** A visitor bounced off
  `/dashboard` signs in and lands on home, one tap from the dashboard card.

- **A 401 demotes the session and does nothing else.** `note_unauthorized` in
  `app.rs` drops the token and sets `SignedOut`; the guard is what moves the
  visitor. It still only transitions from a signed-in state, because writing on
  a 401 with no token to blame would re-run the resource that produced it, fail
  the same way, and write again.

## Alternatives

**`leptos_router`'s `ProtectedRoute`.** It exists on the 0.8 line, and its
`condition: Fn() -> Option<bool>` maps onto the pending state exactly, with
`None` meaning "still loading" — a closer fit than anything that had to be
invented. Rejected on one detail: it renders `<Redirect/>` with no
`NavigateOptions`, and `replace` defaults to `false`, so every rejection pushes a
history entry and the back button bounces between the guarded path and home.
There is no prop to change it. The hand-written component is a few lines, and
those lines are where `replace` lives.

**Preserving the intended destination.** The conventional behaviour, and the
reason it was not adopted is specific rather than principled: the redirect URI
is computed and always lands on `/` (DR-0010), so the OAuth round trip cannot
carry a path and the destination would have to be stashed in `sessionStorage`
across it. That means another stored key, another lifetime to reason about, and
path validation to keep a stored `//evil.example` from becoming an open
redirect. Against that, the dashboard card on the signed-in home is one tap.
Revisit when there are enough screens for a deep link to be how visitors
actually arrive.

**Guarding by redirecting to the hosted UI directly**, skipping the home screen.
Rejected for the reason DR-0010 gives for not redirecting on a 401: it is a loop
the moment the visitor cannot be signed in, and it takes away the choice of
being on the site signed out.

**Rendering a sign-in prompt in place, without navigating.** Tempting, since it
keeps the visitor where they meant to be. Rejected because Page Layouts makes
the signed-out home a focused authentication landing page and the only primary
interaction on it; a second sign-in surface on every guarded screen would
contradict that, and would have to be designed for each screen's layout.

**Leaving the 401 message on the dashboard alongside the guard.** Rejected as
two answers to one event. With demotion driving the guard, the message is
unreachable anyway, and an unreachable branch that looks reachable is worse than
no branch.

## Consequences

Easy: a screen behind the guard may assume a settled auth state and a stored
token, which is why the dashboard's resource no longer watches the auth signal;
new screens get the behaviour by being wrapped, with nothing to restate; and the
guard is visible in the route table, so reading it is enough to see which screens
need a session.

Hard, and accepted:

- **A guarded screen now depends on the guard being in front of it.** The
  dashboard's resource fires immediately, which is correct only because
  `RequireAuth` renders nothing until the state has left `Loading`. Rendering
  `DashboardPage` from an unguarded route would reintroduce the bug the resource
  used to defend against on its own. The comment there says so; nothing enforces
  it.
- **The visitor loses their place on a 401.** They are moved to home rather than
  told what happened where they were, and with no destination preserved, getting
  back is manual. This is the cost of the two decisions above compounding.
- **`Disabled` passing means the guard is inert in ordinary development.** The
  paths that matter most are exercised only under `just dev-web-auth` or against
  a deployed bundle, so a regression in them is not something `just dev-web`
  would reveal.
- **The 401 path cannot be exercised locally at all.** The development API
  validates nothing, so no 401 is producible under `trunk serve`; only API
  Gateway returns one.

Reversing the guard means changing `app.rs` and the routes it wraps. Reversing
the decision not to preserve a destination means adding a stored key and its
validation, and touching nothing else.
