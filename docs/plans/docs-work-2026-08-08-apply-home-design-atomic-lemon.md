# Apply the home design and implement the dashboard

Execution plan for
[`docs/work/2026-08-08-apply-home-design-and-implement-dashboard.md`](../work/2026-08-08-apply-home-design-and-implement-dashboard.md).
That Work Log stays authoritative for the request, the interpretation, and the
assumptions; this file says how the ten steps get carried out.

## Context

The durable design layer already describes three screens — signed-out home,
signed-in home, dashboard — in [Page Layouts](../design/page-layouts.md), the
tokens that style them in [Visual Design](../design/visual-design.md), and the
markup that realizes them in the three references under
[`docs/design/html/`](../design/html/). None of it is running. The SPA today is
an unstyled header, a greeting fetched from `GET /api/greeting`, and an `/about`
page, none of which appears anywhere in the design.

This work turns the design into running code. The API answers with fixed values
so the boundary types and the fetch path are the real ones and only the handler
bodies change when data becomes real. The identity layer is extended so the
display name and profile image the design calls for actually exist as id-token
claims. The greeting demo and `/about` are removed.

Three questions were settled with the user before planning:

- **Undefined screens.** Dashboard rows and the authenticated avatar keep the
  hrefs the design gives them; no route is declared, so the existing `NotFound`
  fallback answers. Nothing about those screens is invented.
- **Boundary shape.** One composite `GET /api/dashboard`, because the screen is
  the unit the endpoint serves and one fetch means one loading and error state.
- **File layout.** `crates/app/src` splits into `app.rs`, `home.rs`,
  `dashboard.rs`, `icons.rs`.

## Steps

### 1. `style/main.css` — the shared visual system

Replace the file wholesale. The three references carry a near-identical
`<style>` block; extract it once, keeping the token names and values recorded in
Visual Design.

Four places where the references disagree with each other, reconciled here:

| Point | References | Resolution |
| --- | --- | --- |
| Wordmark row | home uses `.page-title` alone; dashboard uses `.site-header` with `.wordmark` + avatar | One `.site-header` / `.wordmark` pair. Both are a 42-pixel inline-flex row, so home renders identically with the avatar slot empty. |
| Decorative circles | home has two (`::before`, `::after`); dashboard has one, at a different offset | Keep the home pair. Visual Design describes them in the plural and they are decorative in both. |
| Footer margin | home `0 var(--client-gutter)`; dashboard `var(--space-md) var(--client-gutter) 0` | Home's. `main`'s `padding-bottom: var(--space-md)` already supplies the gap the dashboard rule was adding a second time. |
| `main` | home is a flex column; dashboard is not | Flex column everywhere. The home compositions need `margin-top: auto`; the dashboard is unaffected by it. |

Additions the references have no rule for, all built from existing tokens:

- `.auth-error` — the sign-in failure message above the Google button, in
  `--accent-dark` at `--font-small`. Frontend requires the error be rendered
  with the recovery offered beside it, and the design has no surface for it.
- `.status` — the muted placeholder shown while auth or a fetch is settling.
- `.not-found` — the fallback view, using the page-heading rules.
- `img.profile-image` / `img.account-image` — `object-fit: cover`, since the
  references only ever contain an inline SVG there.

Also drop `.account-email` and its rules: the email never reaches a view.

`index.html` gains `<meta name="theme-color" content="#fffafa">` and its
`<title>` becomes `actord`, which is the product name Page Layouts uses.

### 2. `crates/shared/src/lib.rs` — the boundary types

Remove `Greeting`. Add, all `Serialize + Deserialize + Clone + Debug + PartialEq`:

```rust
pub struct ActionType { pub id: String, pub name: String, pub unit: String, pub icon: String }
pub struct ActionRecord { pub id: String, pub action_type: ActionType, pub value: f64, pub recorded_at: String }
pub struct RecentSummary { pub total: u32, pub daily: Vec<u32> }   // ten days, oldest first
pub struct Dashboard { pub summary: RecentSummary, pub recent: Vec<ActionRecord> }
```

`icon` is an identifier (`"running"`, `"water"`, …), not markup — `shared` is
compiled for both targets and must not carry a view. `recorded_at` is an RFC
3339 string rather than a typed instant, because the crate depends on `serde`
only ([Workspace](../design/workspace.md)) and a date type would break that.
`value` is `f64`, so `5.2 km` and `6200 steps` are the same field.

`total` is the sum of `daily`, not `recent.len()`. Page Layouts states the
ten-day summary and the ten-record list are separate limits, and the dummy data
should demonstrate that rather than hide it.

### 3. `crates/server/src/main.rs` — the dummy handler

Drop `GET /api/greeting` and its handler. Add `GET /api/dashboard` returning a
fixed `Dashboard` built from the reference screen: the ten records it lists
(`Running 5.2 km`, `Water 450 ml`, `Reading 24 pages`, `Meditation 10 min`,
`Cycling 12.4 km`, `Strength training 30 reps`, `Study 45 min`,
`Walking 6200 steps`, `Sleep 7.5 hours`, `Stretching 15 min`) with their
timestamps as RFC 3339, and a ten-element `daily` whose shape follows the
reference bar heights.

`/health` is untouched. No infrastructure change is needed: `GET` is already in
`local.api_methods` and the route is `/api/{proxy+}` (DR-0009).

### 4. `crates/app/src/auth.rs` — provider hint and two claims

- `begin_sign_in` appends `identity_provider=Google` to the authorize query, so
  the button goes to Google rather than to a provider chooser. The `profile`
  scope is already requested, so `SCOPES` is unchanged.
- `IdTokenClaims` gains `name` and `picture`.
- New keys `auth.name` and `auth.picture`, written by `exchange`, read by
  `restore_session`, cleared by `forget_session` alongside `auth.email`.
- `AuthState::SignedIn` carries `name: Option<String>, picture: Option<String>`
  and **not** `email`. The email is still decoded and still written to
  `sessionStorage` — the Work Log keeps it — but a variant field nothing ever
  reads is a `dead_code` warning, and `just lint` denies warnings.

### 5. `infra/identity/main.tf` — the mapping that makes the claims exist

Add to `aws_cognito_identity_provider.google`'s `attribute_mapping`:

```hcl
name    = "name"
picture = "picture"
```

Both are Cognito default standard attributes and the pool declares no explicit
`schema`, so this should be an in-place update. **The AWS session in this
container is expired**, so `terraform plan` and `apply` are yours to run; I will
write the change and gate it on `just tf-fmt-check` and `just tf-validate`,
which need no credentials. Until it is applied, a signed-in build shows the
no-name fallback rather than a display name — worth knowing while reviewing.

### 6. `crates/app/src/api.rs` — the fetch

Remove `fetch_greeting`. Add `fetch_dashboard() -> Result<Dashboard, ApiError>`.
Lift the status-and-decode body the greeting call had into a generic
`get_json<T: DeserializeOwned>(path)` so the 401 separation stays in one place
for every endpoint that follows. `url`, `get`, and `ApiError` are unchanged.

### 7. The screens

**`app.rs`** keeps the `Auth` context and `complete_sign_in` exactly as they are,
and holds the shell: `<Router>` → `.app-shell` → `<main>` → `Routes` → footer.
Routes are `/` (`HomePage`) and `/dashboard` (`DashboardPage`), fallback
`NotFound`. It also holds `SiteHeader`, which takes an optional avatar slot, and
`SiteFooter`.

**`home.rs`** — one route rendering four compositions off `AuthState`:

| State | Renders |
| --- | --- |
| `SignedIn` | eyebrow `Welcome back`, `Hello,` / accented name, lead, account strip (image, name, `Log out`), dashboard card linking `/dashboard` |
| `SignedOut` / `Disabled` | eyebrow `Small actions, real progress`, `Make every` / accented `action count.`, lead, `Continue with Google` |
| `Error` | the signed-out composition with the message in `.auth-error` above the button, which is what makes it recoverable |
| `Loading` | the signed-out heading with a `.status` placeholder where the button goes, so no Google button flashes before the state settles |

A signed-in visitor with no `name` claim gets `Hello, there.`; with no `picture`,
the avatar falls back to a generic glyph. Both are reachable today, and both stay
reachable for anyone who signs in with a Google account carrying neither.

**`dashboard.rs`** — the header avatar, the page heading, and a `LocalResource`
over `fetch_dashboard` inside `<Suspense>`. Three things move here from the
current `HomePage` unchanged in substance:

- the auth signal read inside the resource's source closure, which is what
  re-runs the fetch once `complete_sign_in` has stored the token;
- the `Effect` that drops the session on a 401, with its guard against looping;
- the rendered error branch, never swallowed.

Bar heights are `count / max * 100` percent as an inline style, the last bar in
`--white`, the whole chart `aria-hidden` with the total in text beside it.
`recorded_at` renders as `YYYY-MM-DD HH:MM` in local time via `js_sys::Date`
(already a dependency), falling back to the raw string if the browser parses it
as `NaN`. Rows link to `/actions/new?action_type={id}` and the avatar to
`/action-types`; neither route exists, so `NotFound` answers — the settled
choice, recorded in the Work Log with its reason.

**`icons.rs`** — every inline SVG: the wordmark mark, the Google mark, the arrow,
the plus, the ten activity glyphs matched on `ActionType::icon` with a fallback
for an unknown id, and the avatar fallback.

`main.rs` gains the three `mod` declarations.

### 8. Comments and stale references

`Trunk.toml` and `infra/api/apigateway.tf` both name a route that no longer
exists in explanatory comments; update the example, not the rule. Decision
Records are append-only and their mentions of `/api/greeting` are historical
measurements — leave them alone.

### 9. Verify

I run, in the container:

- `just fmt-check`, `just lint`, `just check`, `just test`, `trunk build`
- `just tf-fmt-check`, `just tf-validate`
- `just dev-api` and `curl -s localhost:3000/api/dashboard | jq`, to confirm the
  payload shape and that `/api/greeting` is gone

The devcontainer has no browser ([Workspace](../design/workspace.md)), so the
visual pass is yours: `just dev-api` and `just dev-web` together, then port 8080
at 320 and 430 pixels wide and at a height of 720 or less, plus tab-through focus
rings and a reduced-motion run. `just dev-web-auth` exercises the signed-in home
and the Google provider hint, and needs the step 5 apply first for the name and
image to appear. I will drive it with the `claude-in-chrome` skill instead if you
would rather, and report what it shows.

### 10. Documents

Append dated Progress entries to the Work Log as the work lands, including the
step-8 decision and its reason, and fill its Verification section.

Then **draft, and present for confirmation without applying** — Design Documents
are overwritten by nature and `docs/README.md` reserves that for a human:

- [Frontend](../design/frontend.md) — the file table, the routes, the
  `Consumes` line, the auth claims and `AuthState` shape, and the sentence
  about a deployed unconfigured build "rendering a 401 where the greeting
  belongs".
- [Page Layouts](../design/page-layouts.md) — drop the email from the signed-in
  home's account strip, from the data table's first row, and lose the sentence
  about it truncating on one line.
- [index.md](../design/index.md) — the backend paragraph still describes
  `GET /api/greeting` as the API's whole surface.
- [deployment.md](../design/deployment.md) — one sentence names the greeting
  endpoint as what a fresh CloudFront serves a 401 for.

Decision Record candidates stay listed in the Work Log for `/work-done` to judge;
this plan does not write any.

## Order

2 → 3 → 6 gets the boundary compiling end to end before any view exists. 1 → 7
is the bulk. 4 and 5 are independent of both and can land at any point. 8 is
cleanup, 9 gates, 10 closes.
