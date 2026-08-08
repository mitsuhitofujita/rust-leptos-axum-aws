# Route guards for authenticated screens — implementation plan

Working record for `docs/work/2026-08-08-route-guards.md`. That Work Log is the
document this project's model recognises; this file is a plan-mode artefact and
should not be committed.

## Context

Authentication landed in the previous unit of work: `crates/app/src/auth.rs`
runs Authorization Code Flow with PKCE against the Cognito hosted UI (DR-0010),
and `app.rs` settles an `AuthState` into a context signal once at mount. Nothing
yet uses that state to decide **where a visitor may go**.

Today `/dashboard` is reachable while signed out. The screen mounts, fetches
`/api/dashboard`, and — against a deployed API — gets a 401 from API Gateway's
authorizer, which `dashboard.rs` turns into an inline "sign in and try again"
message. The visitor is left on a broken screen and told to fix it from
somewhere else. As the action-type and action screens arrive, that failure mode
would be repeated on each of them.

The guard being added is a **user-experience mechanism, not a security
boundary**. API Gateway's JWT authorizer remains the only enforcement point
(DR-0010, `infra/api/apigateway.tf:34`); the guard exists so an unauthenticated
visitor never lands on a screen that can only fail.

The related question of where a Cognito session is best held resolved to "no
change": `sessionStorage` with no refresh token is already the right choice for
a bundle served as static files with no session-holding server. No code follows
from it.

## Approach

### 1. `RequireAuth`, in `crates/app/src/app.rs`

`app.rs` already owns the `Auth` context, `auth_state()`, `SiteHeader` and
`NotFound`, and the guard is part of the router, so it lives there too.
`auth.rs` stays free of Leptos.

The guard maps the five `AuthState` values onto three outcomes through a
`Memo`, so that an auth-state change which does not change the outcome does not
re-render — without the memo, `children()` would be rebuilt on every signal
write and `DashboardPage` would remount along with its resource.

| `AuthState` | Outcome | Why |
| --- | --- | --- |
| `SignedIn` | `Allow` | |
| `Disabled` | `Allow` | Unconfigured build; blocking it would put `just dev-web` behind a sign-in that does not exist (DR-0008) |
| `Loading` | `Pending` | `complete_sign_in` may be mid-exchange; rejecting would eject a visitor who is in the middle of returning |
| `SignedOut` | `Deny` | |
| `Error(_)` | `Deny` | `home.rs:41` already renders the message alongside a retry |

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum Access { Allow, Pending, Deny }

#[component]
pub fn RequireAuth(children: ChildrenFn) -> impl IntoView {
    let auth_state = auth_state();
    let access = Memo::new(move |_| match auth_state.get() { /* table above */ });

    // A navigation is a side effect, so it runs in an `Effect` rather than
    // during render. `replace: true` is the point: the default pushes a history
    // entry, and the back button would bounce between here and home.
    let navigate = use_navigate();
    Effect::new(move || {
        if access.get() == Access::Deny {
            navigate("/", NavigateOptions { replace: true, ..Default::default() });
        }
    });

    move || match access.get() {
        Access::Allow => children(),
        Access::Pending => view! {
            <SiteHeader />
            <p class="status">"Checking your session…"</p>
        }.into_any(),
        // The effect above is already leaving; rendering nothing avoids a
        // flash of the pending copy on the way out.
        Access::Deny => ().into_any(),
    }
}
```

Two details that will otherwise cost a compile cycle:

- `children` must be `ChildrenFn`, not `Children`. `Children` is `FnOnce` and
  the guard re-renders.
- `use_navigate` is `leptos_router::hooks::use_navigate`; `NavigateOptions` is
  `leptos_router::NavigateOptions`.

**`leptos_router::components::ProtectedRoute` is deliberately not used.** It
exists in 0.8.15 and its `condition: Fn() -> Option<bool>` maps onto the
pending state neatly, but `components.rs:437` renders `<Redirect path=.../>`
with no `NavigateOptions`, and `NavigateOptions::default().replace` is `false`
(`navigate.rs:23`) — so every rejection pushes a history entry.

### 2. Apply it in the route table

```rust
<Route path=path!("/dashboard") view=|| view! { <RequireAuth><DashboardPage/></RequireAuth> } />
```

`/` stays unguarded: `docs/design/page-layouts.md` defines signed-out and
signed-in home as two states of one screen. No reverse guard either — a
signed-in visitor arriving at `/` is not redirected to `/dashboard`.

The routes a dashboard row and the account control link to (`/actions/new`,
`/action-types`) are not declared at all, so `NotFound` still answers them and
they are guarded when they are built, not before.

### 3. Give the 401 one job

`app.rs` gains the shared demotion, holding the loop guard that
`dashboard.rs:37-44` currently carries:

```rust
/// What an unexpected 401 does: drop the session and let the guard move the
/// visitor. Only a signed-in state transitions — a 401 arriving with no token
/// to blame must write nothing, because writing would re-run the resource that
/// produced it.
pub fn note_unauthorized() { /* forget_session + set SignedOut, guarded */ }
```

In `crates/app/src/dashboard.rs`:

- The effect body becomes a call to `note_unauthorized()`.
- The `Err(ApiError::Unauthorized)` view branch (`dashboard.rs:78-83`) is
  deleted. It is unreachable once demotion triggers the guard, and `Err(error)`
  already covers the arm exhaustively.
- The resource loses its `let _ = auth_state.get();` source read and becomes
  `LocalResource::new(fetch_dashboard)`. Its comment is rewritten to state the
  new precondition — the guard renders nothing until the state has left
  `Loading`, so the token is stored before this screen exists. The existing
  comment asserts the opposite is load-bearing, and leaving a false rationale in
  place is worse than the coupling this introduces.

## Files

| File | Change |
| --- | --- |
| `crates/app/src/app.rs` | `Access`, `RequireAuth`, `note_unauthorized`; `/dashboard` wrapped |
| `crates/app/src/dashboard.rs` | Effect calls the shared demotion; `Unauthorized` branch removed; resource decoupled from the auth signal |
| `docs/decisions/DR-0011-*.md` | New record (draft) |
| `docs/design/frontend.md` | Routing and Authentication sections (draft, needs confirmation) |
| `docs/design/page-layouts.md` | One line in Navigation (draft, needs confirmation) |
| `docs/work/2026-08-08-route-guards.md` | Progress and Retirement |

`auth.rs`, `api.rs`, `home.rs` and `crates/server` are untouched.

## Verification

Automated: `just fmt`, `just lint`, `just check`. `just lint` runs clippy for
both the host target and `wasm32-unknown-unknown` with `-D warnings`.

By hand, with `just dev-api` running:

1. **`just dev-web`** (unconfigured, `AuthState::Disabled`) — `/dashboard`
   renders. This is the regression the guard could most easily cause.
2. **`just dev-web-auth`, signed out** — enter `/dashboard` in the address bar.
   The address settles on `/` and the signed-out home renders. Press back: it
   must leave the site rather than return to `/dashboard`, which is what
   `replace: true` buys.
3. **`just dev-web-auth`, sign in from `/`** — hosted UI, return to `/`,
   signed-in home, dashboard card, dashboard renders.
4. **Signed in, reload on `/dashboard`** — briefly "Checking your session…",
   then the dashboard. No ejection to `/` during the `Loading` window.
5. **Sign out from home, then back-button toward `/dashboard`** — the guard
   returns the visitor to `/`.

**Known gap:** the 401 path cannot be exercised locally. The dev API validates
nothing, so no 401 is producible under `trunk serve`; only API Gateway returns
one. It will be reasoned about rather than observed, unless the change is
deployed. This is recorded in the Work Log rather than papered over.

## Notes for the durable layer

DR-0011 should carry, because none of it survives in a Design Document:

- The guard is user experience, not authorization — stated plainly, so nobody
  later reads it as a reason to skip server-side checks.
- Why `Loading` holds and `Disabled` passes.
- Why `ProtectedRoute` was rejected (the `replace` default).
- Why the return destination is not preserved: `redirect_uri` is
  `origin + "/"` and computed rather than configured (DR-0010), so the OAuth
  round trip cannot carry a path; preserving one would mean a stored
  `auth.return_to` and open-redirect validation, which the dashboard card being
  one tap from home does not justify.
- Why the 401 no longer redirects on its own, and what that removed.
- That the memo, not the raw signal, is what keeps guarded screens from
  remounting.

Separately: `docs/design/frontend.md`'s file table lists only `main.rs`,
`app.rs`, `api.rs` and `auth.rs`, and has been stale since `home.rs`,
`dashboard.rs` and `icons.rs` were added. The frontend.md edit corrects it while
that section is open.
