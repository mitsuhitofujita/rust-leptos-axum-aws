# Frontend

Updated: 2026-08-19

## Purpose

The user interface: a Leptos single-page application, rendered entirely in the
browser, that fetches its data from the axum API over HTTP. It is delivered as
static files and runs no server-side code of its own (DR-0001).

## Structure

The application lives in `crates/app` and is compiled to
`wasm32-unknown-unknown`. It is a binary crate — trunk builds `src/main.rs`.

| File | Role |
| --- | --- |
| `crates/app/src/main.rs` | Installs the panic hook, mounts `App` to `<body>` |
| `crates/app/src/app.rs` | The router, the shared shell, the auth context and the route guard |
| `crates/app/src/home.rs` | `/` — the signed-out and signed-in compositions of one screen |
| `crates/app/src/dashboard.rs` | `/dashboard` — the ten-day summary and the recent records |
| `crates/app/src/action_types.rs` | `/action-types`, `/action-types/new` and `/action-types/:id` — the index, registering one, and editing or deleting one |
| `crates/app/src/actions.rs` | `/actions`, `/actions/new` and `/actions/:id` — the history, recording one, and editing or deleting one |
| `crates/app/src/icon_picker.rs` | The icon field: the compact selector and the modal it opens |
| `crates/app/src/type_picker.rs` | The action-type field on the create-action form: the compact selector and the modal it opens — the same shape as `icon_picker.rs`, choosing a registered type instead of an icon |
| `crates/app/src/icon_catalog.rs` | The supported icons. Generated — see below |
| `crates/app/src/icons.rs` | The inline SVGs that are not action-type icons, and the wrapper that draws one |
| `crates/app/src/format.rs` | Display formatting for an action record's value and timestamp, shared by the dashboard and the actions screens |
| `crates/app/src/api.rs` | Calls to the API, returning `shared` types |
| `crates/app/src/auth.rs` | Sign-in against the Cognito hosted UI, and the token it yields |

Trunk's inputs sit at the repository root rather than inside the crate, because
the workspace manifest is virtual and cannot itself be a trunk target:

| File | Role |
| --- | --- |
| `index.html` | trunk entry point; points `data-trunk rel="rust"` at `crates/app/Cargo.toml` |
| `Trunk.toml` | dev server address and port. The `/api` proxy is *not* here — see below |
| `style/main.css` | plain CSS, linked by `index.html` |
| `public/` | assets copied verbatim into `dist/public/` |

`trunk build` emits `dist/`: hashed `.wasm` and `.js`, hashed CSS, the copied
`public/` directory, and an `index.html` rewritten to reference them. `dist/` is
the deployable artefact and is not committed.

**Routing.** `leptos_router`'s `<Router>` wraps a `<Routes>` block declaring
`/` (`HomePage`), `/dashboard` (`DashboardPage`), `/action-types/new`
(`NewActionTypePage`), `/action-types` (`ActionTypesPage`),
`/action-types/:id` (`EditActionTypePage`), `/actions/new` (`NewActionPage`),
`/actions` (`ActionsPage`) and `/actions/:id` (`EditActionPage`), with a
`NotFound` fallback. Navigation uses `<A>`, which renders `aria-current="page"`
on the active link — the CSS styles that attribute rather than tracking the
active route by hand.

`/actions/new` reads an optional `?action_type=` query parameter through
`leptos_router::hooks::use_query_map`, preselecting that type when it names
one the account actually has — the dashboard's repeat link is what sends it,
and page-layouts.md requires the preselection survive the transition.

**Route guarding.** `/dashboard` is wrapped in `RequireAuth`, and every screen
added behind a session is wrapped the same way, so the route table is where the
requirement is visible. The guard maps the five auth states onto three outcomes:
`SignedIn` and `Disabled` render the screen, `Loading` holds with the same
"Checking your session…" copy the home screen shows, and `SignedOut` and `Error`
redirect to `/` — replacing the history entry rather than pushing one. The
mapping goes through a `Memo` so a state change that leaves the outcome alone
does not rebuild the screen behind it (DR-0011).

`/` is not guarded in either direction. Authentication changes what it renders
and never where the visitor is, and a signed-in visitor arriving there stays
there.

The guard decides nothing about access. The service's own Cognito verification
is the only enforcement point (DR-0028); `RequireAuth` exists so an
unauthenticated visitor never lands on a screen that can only fail (DR-0011).

**Data fetching.** `crates/app/src/api.rs` holds one async function per
endpoint. Each returns `Result<T, ApiError>` where `T` comes from
`crates/shared`. `ApiError` separates `Unauthorized` from everything else,
because 401 is the one failure a visitor can act on; the rest are transport
failures, unexpected statuses and decode failures carried as messages the UI can
render. Every request attaches the access token when the tab holds one.
Components load them with `LocalResource` inside `<Suspense>`; the error branch
is rendered, never swallowed.

`LocalResource`, not `Resource`: the browser fetch future is not `Send`, and in
a CSR build there is no server to run it on.

A refused request answers with its reason in the body, and that reason is what
the screen shows — a form reporting `400` where it could report
`A numeric unit is required.` is reporting the wrong thing. The status stands in
only when the body is empty.

Writes do not go through a resource. `create_action_type`, `update_action_type`
and `delete_action_type` are each called from an event handler in
`spawn_local`, with the screen holding its own `saving`/`deleting` and `error`
signals, because a submission is something the visitor did once rather than
something the screen is a view of. `actions.rs`'s `create_action_record`,
`update_action_record` and `delete_action_record` follow the same shape;
`update_action_record` sends only a value, since DR-0016 fixes everything
else about a record once it is created.

**Deletion** is confirmed by a second native `<dialog>`, shown with
`showModal` the same way `IconField`'s picker is (DR-0013) — focus
containment and Escape-to-close come from the browser rather than being
written by hand. `Keep action type` (`Keep action`, on the actions screens)
is first in document order and holds initial focus, so an accidental Escape
or Enter does not land on the destructive action.

**The account menu** is `app.rs`'s `AccountControl`, the control every
authenticated screen carries at the top row's end (DR-0029). It is a third
native `<dialog>`, opened the same way as the two above: `showModal`, an
`on:close` handler returning focus to the trigger, nothing hand-rolled for
focus containment or Escape. It holds `Dashboard` (`/dashboard`), `Action`
(`/actions`), `Action Type`, a separator, and `Log out`, which calls the same
`auth::sign_out()` the signed-in home uses.

**Icons.** An action type's icon is a canonical kebab-case Lucide name, and
`icon_catalog.rs` is what turns one into a glyph. It is generated by
`just icons` — the table of names, official English names, and the geometry of
each `<svg>` — and must not be edited (DR-0014, DR-0019). `icons::Glyph` writes
the wrapper those children go in; `icons::ActivityGlyph` looks a name up and
falls back to a generic glyph when the catalog does not know it, which it can,
because the name arrives over the wire and the catalog belongs to the build.

Everything else in `icons.rs` — the Google mark, the arrows, the plus, the
avatar stand-in — is hand-written, because drawing a chevron from the catalog
would mean admitting a whole further category of icons nobody may choose.

**The icon picker** is `icon_picker::IconField`, one component holding both the
compact selector and the `<dialog>` it opens. Native throughout: `showModal`
keeps focus inside and closes on Escape, the search field does its own editing,
and the radio group does its own arrow keys. What is written is the filtering,
the live count, and the rule that a staged choice reaches the form only when
`Use selected icon` is activated (DR-0013).

**The type picker** is `type_picker::TypeField`, the same shape as `IconField`
generalized from choosing an icon to choosing one of the account's own
registered action types on the create-action form. It differs in one respect:
`IconField` reads a 725-row compile-time catalog and defers building its rows
until the dialog first opens, because that catalog is large enough to make the
deferral worth writing; `TypeField` reads a list the page above it already
loaded and small enough to render eagerly, so it has no such deferral. The
edit-action screen does not use it — a record's type is read-only there
(DR-0016) — and shows the same icon, name and unit as plain text instead.

The 725 rows are built the first time the dialog opens rather than at first
paint, so a visitor who keeps the icon they were given never pays for them.
Filtering hides rows rather than rebuilding the list, so a keystroke does not
rebuild 725 SVGs.

**Authentication.** `crates/app/src/auth.rs` implements Authorization Code Flow
with PKCE against the Cognito hosted UI by hand — no auth library, no AWS SDK
(DR-0010). `App` settles an `AuthState` signal once at mount and provides it
through context: `Loading` until the callback has been dealt with, then
`Disabled`, `SignedOut`, `SignedIn` or `Error`. The header renders a control from
it, and nothing at all when the state is `Disabled`.

The access token, its expiry and the claims read for display live in
`sessionStorage`. No refresh token is kept: an expired session sends the visitor
back to the hosted UI. The signal is what screens render from; storage is only
what lets a reload recover the session.

A 401 drops the session and nothing more. `note_unauthorized` in `app.rs` is the
one place that does it, and the guard is what then moves the visitor to a home
screen offering a fresh sign-in. Nothing redirects to the hosted UI on its own,
which would loop for any 401 a new token cannot fix (DR-0010). Only a signed-in
state transitions, so a 401 arriving with no token to blame changes nothing.

The redirect URI is not configured — it is `window.location.origin` with a
trailing slash, so it is the CloudFront domain in a deployed build and
`http://localhost:8080/` under `trunk serve`, both already registered on the app
client.

A guarded screen may assume a settled auth state. `RequireAuth` renders nothing
while the state is `Loading`, so the token is already stored by the time the
screen exists and its resource can fire immediately, without watching the auth
signal to find out when it is safe to. That holds only for screens behind the
guard — rendering one from an unguarded route would send its first request
before the token is there (DR-0011).

**Compile-time configuration** is three environment variables, each read once
through `option_env!` into a constant, and each with an unset value that means
something workable rather than something broken (DR-0008). `just deploy-web`
resolves all three from SSM; `just dev-web-auth` resolves the two Cognito ones
around `trunk serve`.

| Variable | Unset means | Read by |
| --- | --- | --- |
| `API_BASE_URL` | the empty string, so every call stays relative and the trunk proxy serves it | `api.rs` |
| `COGNITO_CLIENT_ID` | sign-in is not configured | `auth.rs` |
| `COGNITO_HOSTED_UI_DOMAIN` | the same | `auth.rs` |

Either Cognito variable empty disables sign-in entirely: no control, no
`Authorization` header, and the local API — which validates nothing — answers
anyway. That is what keeps development needing no configuration at all.

## Interfaces

**Consumes** `GET /api/dashboard`, `GET`/`POST /api/action-types`,
`GET`/`PUT`/`DELETE /api/action-types/{id}`, `GET`/`POST /api/actions` and
`GET`/`PUT`/`DELETE /api/actions/{id}` — see [Backend](backend.md) — as
absolute paths joined to `API_BASE_URL`, carrying a bearer token when there is
one. No API hostname appears in the source; the origin arrives at build time,
or not at all in development.

**Consumes** the Cognito hosted UI's `/oauth2/authorize`, `/oauth2/token` and
`/logout`, at the domain `COGNITO_HOSTED_UI_DOMAIN` names.

**Depends on** `leptos` (feature `csr`), `leptos_router` (default features — it
has no `csr` feature), `gloo-net` (features `http`, `json`) for fetch,
`console_error_panic_hook`, and `shared`. The sign-in flow adds `web-sys`
(features `Window`, `Location`, `History`, `Storage`, `Crypto`,
`UrlSearchParams`), `js-sys`, `wasm-bindgen`, `sha2` and `base64` for the PKCE
challenge, and `serde` with `serde_json` for the token response. Every one was
already in `Cargo.lock` at the version the workspace now names, so adding them
moved nothing — `wasm-bindgen` least of all, which trunk keys its CLI download
off (DR-0003).

The icon picker adds four more `web-sys` features — `HtmlDialogElement`,
`HtmlButtonElement`, `HtmlElement` and `HtmlInputElement` — for the three calls
Leptos does not make for it: `showModal`, `close`, and `focus`.

**Does not depend on** `lucide-leptos`, although every action-type icon is one
of its. The geometry arrives as the generated `icon_catalog.rs`; the crate is a
dependency of `crates/icongen` alone (DR-0019).

**Exposes** nothing to other crates.

## Constraints

- No API hostname is written into the source. Calls are absolute paths joined to
  `API_BASE_URL`, which is supplied at build time and not fetched at runtime, so
  the development proxy and the production origin are both settled outside the
  code — DR-0008.
- The dev server proxies `/api` so that development is single-origin and CORS
  never arises. Production is cross-origin and requires CORS on the API —
  DR-0001. Which backend it proxies to is chosen by the recipe rather than by
  `Trunk.toml`, which holds no `[[proxy]]` block: `dev-web` and `dev-web-auth`
  both pass `127.0.0.1:3000`, the service itself, because trunk appends a
  command-line backend to the file's entries rather than overriding them, so a
  default in the file could not be overridden by the flag either recipe needs
  to pass regardless — DR-0023. A bare `trunk serve` outside `just` therefore
  proxies nothing.
- Every request under `/api` needs a Cognito access token in an `Authorization`
  header, which `auth.rs` obtains from the hosted UI and `api.rs` attaches — but
  only in a build configured for it. An unconfigured build sends no header,
  which `just dev-api`'s `identity::Auth::Mock` accepts and the deployed
  `identity::Auth::Cognito` does not, so a deployed bundle built without the
  two Cognito variables fails every call it makes. Configuration is what
  distinguishes the two, not code — DR-0010.
- The token is never validated in the browser. Expiry is checked so an expired
  one is not sent, and the id token is decoded once for the claims the screens
  display, but neither is a signature check: the service's own Cognito
  verification is the security boundary — DR-0010, DR-0028.
- The route guard is not part of that boundary. It keeps a visitor off a screen
  that could only fail, and decides nothing a server would otherwise decide —
  DR-0011.
- A screen behind `RequireAuth` may assume a settled auth state and a stored
  token. Rendering one from an unguarded route reintroduces the race the guard
  removes — DR-0011.
- Deep links are router paths, not files. `trunk serve` serves `index.html` for
  unknown paths, and the production host must be configured to do the same, or
  reloading on any non-root route fails — DR-0001.
- CSS is plain and hand-written. No framework, no Node.js, no npm anywhere in
  the build.
- The framework stays inside this crate; `shared` and `server` never import
  Leptos — DR-0002.
- `src/icon_catalog.rs` is generated and committed. Editing it by hand is
  undone by the next `just icons`, and nothing runs that automatically —
  DR-0019.
- Client-side validation is a convenience, never the check. The service
  validates what it stores, and the form's own rules exist to save a round trip
  — see [Backend](backend.md).
- A numeric field bound with `prop:value` (a controlled input, re-applying the
  signal to the DOM on every keystroke) must use `type="text"` with
  `inputmode="decimal"` or `"numeric"`, never `type="number"`. A `type="number"`
  input's `value` goes empty the instant its text stops being a valid
  floating-point number — which a bare trailing `.` is not — so the reactive
  write-back erases the `.` before a fractional digit can follow it, making
  decimal entry impossible. The action Value field learned this the hard way
  — DR-0034.
- A `<dialog>`'s CSS must not give it an unconditional `display` value. The
  browser's own stylesheet is what hides one after `close()` —
  `dialog:not([open]) { display: none; }` — and author styles outrank that
  regardless of specificity, so a bare `.some-dialog { display: flex; }`
  keeps overriding it even once `open` is gone, leaving the dialog visible
  with a working `close()` underneath it. Any `display` a dialog needs
  belongs on its `[open]` variant instead, alongside its open-state
  animation — the account menu's close button learned this the hard way.
