# Apply the home design and implement the dashboard

Status: in progress
Started: 2026-08-08
Branch: main

## Request

Apply the documented home page design to the running SPA, and implement the
dashboard screen. For this step the API answers with dummy responses rather than
real data.

### Clarifications

- The dummy data lives in the axum server. `crates/server` gains the endpoints
  and returns hardcoded JSON; the SPA fetches them through `api.rs` exactly as it
  would fetch real data, so only the handler bodies change later.
- Display name and profile image come from the identity layer. `infra/identity`
  extends the Google attribute mapping so `name` and `picture` reach the user
  pool and appear as id-token claims, and `auth.rs` reads them alongside `email`.
- The signed-out home's `Continue with Google` button passes
  `identity_provider=Google` to the hosted UI, so the button goes straight to
  Google rather than to a provider chooser.
- The existing greeting demo and the `/about` page are removed: the route,
  `GET /api/greeting`, `shared::Greeting`, and the UI that renders them.
- The email address is never displayed. The only account identity shown on
  screen is the display name and the profile image. The email may still be held
  internally, although nothing uses it at present.

## Interpretation

**What is being asked.** Turn the durable design into running code. Today the SPA
renders an unstyled router with a header, a greeting fetched from the API, and an
about page — none of which appears in [Page Layouts](../design/page-layouts.md).
After this work the SPA presents three of the documented screens: signed-out
home, signed-in home, and dashboard, styled by the tokens in
[Visual Design](../design/visual-design.md), with the dashboard fed by API
responses whose values are fixed but whose shape is the intended one.

**Scope.** The three screens with an HTML reference under
[`docs/design/html/`](../design/html/); the CSS that realizes the shared visual
system; the routing change that adds the dashboard and drops `/about`; the
`shared` types for action types and action records; the dummy handlers behind
them; the `api.rs` calls that fetch them; the auth changes for the Google
provider hint and the two new claims; and the Terraform attribute mapping that
makes those claims exist.

**Out of scope.** The five screens listed in Page Layouts as having no HTML
reference — action types, add action type, edit action type, actions, add action.
Their layouts are undefined and Page Layouts is explicit that they must not be
inferred from the home and dashboard references. Dashboard rows therefore link to
a route that does not yet render its screen; the plan settles how that is handled
rather than inventing the screen. Persistence, a database, and any real query
behind the dummy handlers are also out of scope, as is authorizing the new
endpoints differently from the existing ones.

**Assumptions.**

- The dashboard route is `/dashboard`. No document names it; the signed-in home's
  dashboard card and the navigation diagram in Page Layouts establish that the
  screen is reachable from home, not that it lives at a particular path.
- Home is one route, `/`, that renders the signed-out or signed-in composition
  from `AuthState`. Page Layouts describes them as two states of one screen, and
  the constraint that authentication changes the home layout rather than
  producing a separate visual system reads as one route.
- `AuthState::Disabled` — a build with no Cognito configuration — renders the
  signed-out home. It is the only composition that makes sense without an
  identity, and it keeps `just dev-web` usable with no configuration at all,
  which [Frontend](../design/frontend.md) treats as load-bearing.
- The dashboard is only meaningful when signed in, but nothing in the durable
  layer defines a redirect or a guard. It will render for an unauthenticated
  visitor and let the API's 401 surface through the existing `ApiError` path,
  rather than introducing a route guard this request did not ask for.
- The ten-day chart is supplemental and hidden from assistive technology, so the
  dummy summary supplies per-day counts and a total; the bars carry no accessible
  content of their own.
- The three HTML references are authoritative for markup structure and styling,
  and their inline `<style>` blocks are the source for `style/main.css`. Where
  they disagree with the two Design Documents, the Design Documents win, since
  they are the durable layer and the references are cited as illustrations of it.
- `username` in the clarification means the display name — Google's `name` claim,
  mapped into the pool by step 5. It is not Cognito's username attribute, which
  is Google's `sub` and is a numeric identifier with nothing displayable in it.
- Dropping the email from the display contradicts
  [Page Layouts](../design/page-layouts.md), which currently puts profile image,
  display name, email, and `Log out` in the signed-in home's account strip, and
  states that the email truncates on one line. Both sentences are now wrong and
  the document needs updating, which step 10 covers. The account strip keeps its
  remaining three elements rather than being removed.
- The email is still read from the id token and stored in `sessionStorage`,
  because `auth.rs` already does so and removing it is a change this request did
  not ask for. It simply never reaches a view.
- The user pool declares no explicit `schema` block, so `name` and `picture` are
  already part of Cognito's default standard schema. Adding them to
  `attribute_mapping` should not replace the pool, whose `prevent_destroy` and
  `deletion_protection` would block that anyway. The plan verifies this against a
  Terraform plan before applying.

## Plan

1. Read the three HTML references in full and extract their shared `<style>`
   block into `style/main.css`, keeping the token names and values that
   [Visual Design](../design/visual-design.md) records.
2. Define the boundary types in `crates/shared`: an action type carrying name,
   unit, and an icon identifier; an action record carrying its action type, a
   numeric value, and a recorded timestamp; and the dashboard payload combining
   the ten-day summary with the ten most recent records.
3. Add the dummy handlers to `crates/server` behind the paths those types imply,
   returning fixed values that match the reference screens. Remove
   `GET /api/greeting`.
4. Extend `crates/app/src/auth.rs`: pass `identity_provider=Google` on the
   authorize request, decode the `name` and `picture` claims from the id token,
   store them beside the email, and widen `AuthState::SignedIn` to carry them.
5. Extend `infra/identity` to map Google's `name` and `picture` into the user
   pool. Confirm with `terraform plan` that the pool is updated in place before
   applying.
6. Add the fetch functions to `crates/app/src/api.rs` and remove
   `fetch_greeting`.
7. Rebuild `crates/app/src/app.rs` around the documented screens: the shared
   shell, top row, page heading, and footer; the signed-out and signed-in home
   compositions; and the dashboard. Add the `/dashboard` route and remove
   `/about` and the greeting UI. Split the file if it stops being readable as
   one — Frontend currently documents `app.rs` as holding every component, and
   that description is updated if it changes.
8. Settle what a dashboard row links to, given that add-action has no defined
   screen. Record the choice and its reason here.
9. Verify: `just fmt-check`, `just lint`, `just check`, `just test`,
   `just tf-fmt-check`, `just tf-validate`, a `trunk build`, and the three
   screens exercised against `just dev-api` in a browser at 320, 430, and a
   height at or below 720 pixels, plus a keyboard-focus and reduced-motion pass.
10. Draft the updates to [Frontend](../design/frontend.md) — routing, the file
    table, data fetching, the auth claims — and to
    [Page Layouts](../design/page-layouts.md), which must at minimum drop the
    email from the signed-in home's account strip and from the data table's
    first row, and lose the sentence about the email truncating on one line.
    Update it further if implementation settles anything else it leaves open.
    Present the drafts for confirmation; do not overwrite before that.

## Progress

### 2026-08-08

- Read `docs/README.md`, the Design Document index, Page Layouts, Visual Design,
  and Frontend, plus the current `app.rs`, `api.rs`, `main.rs`, `shared`, and
  `infra/identity/main.tf`.
- Found that the Google identity provider maps only `email` and `username`, so
  the display name and profile image the design calls for do not exist in the id
  token today. Raised it with the user, who chose to extend the identity layer
  rather than derive a name from the email address.
- Wrote this log. No code changed yet; awaiting confirmation of the
  interpretation and plan.
- The user then removed the email from the display. This is the first point at
  which the request overrides the durable design rather than realizing it: Page
  Layouts requires the email in the signed-in home's account strip. The
  requirement is now retired, and the account strip carries profile image,
  display name, and `Log out`.

**Decision Record candidates**, to be judged as the work settles rather than now:

- Whether the dashboard is served by one composite endpoint or by separate
  summary and recent-actions endpoints. The choice fixes the boundary contract
  and is awkward to reverse once the SPA and the API both depend on it.
- Serving fixed responses from the real API rather than embedding dummy data in
  the SPA. The alternative was considered and rejected; the reason — that only
  the handler bodies change when real data arrives — has no home in a Design
  Document.
- Sending `identity_provider=Google` from the SPA. It ties a hosted-UI detail to
  a button label and would need revisiting if a second provider is ever added,
  which DR-0010 does not currently cover.

The interpretation and plan were then confirmed, and three questions settled
before implementation began. The execution plan is
`docs/plans/docs-work-2026-08-08-apply-home-design-atomic-lemon.md`.

- **Step 8, settled.** A dashboard row links to
  `/actions/new?action_type=<id>`; no route is declared, so the router's
  `NotFound` fallback answers. The same question turned out to apply twice, not
  once: Page Layouts also requires the authenticated avatar to reach the
  action-type area, so it links to `/action-types` under the same rule. The
  reason for the choice is that a placeholder screen would be a product decision
  about layouts Page Layouts explicitly refuses to let implementation infer,
  while dropping the links would contradict the requirement that the row and the
  avatar *are* targets. Keeping the intended href and letting a fallback answer
  is the only option that neither invents nor removes anything. The fallback's
  copy was widened to say the screen may not be built yet, so the dead end reads
  as unfinished rather than broken.
- **Boundary shape, settled.** One composite `GET /api/dashboard` rather than
  separate summary and recent-actions endpoints. The dashboard is the unit the
  endpoint serves, and one response is one loading state and one error state.
- **File layout, settled.** `app.rs` no longer holds every component: it keeps
  the router, the shell, the shared top row and the fallback, and `home.rs`,
  `dashboard.rs` and `icons.rs` hold the screens and the SVGs. Three screens
  plus ten inline glyphs do not read as one file.

Implementation notes worth keeping:

- The three HTML references disagree with each other in four places, all of them
  incidental rather than intentional: home carries the wordmark as `.page-title`
  and the dashboard as `.site-header` + `.wordmark`; home draws two decorative
  circles and the dashboard one, at a different offset; the footer margin differs
  by 16 pixels; and only home's `main` is a flex column. Each was resolved
  towards the Design Documents and the reconciliation recorded in the plan. One
  `.site-header` with an optional trailing control now serves both, which is also
  what Page Layouts describes.
- `AuthState::SignedIn` carries `name` and `picture` but **not** `email`. Keeping
  the address on the variant would be a field nothing reads, which `just lint`
  denies as dead code. The email is still decoded from the id token and still
  written to `sessionStorage`, as the request asked; it simply has no reader.
- `RecentSummary::total` is the sum of the daily counts, not the length of the
  recent list — the dummy data returns 50 against 10 records, so the two limits
  Page Layouts calls separate are visibly separate rather than accidentally
  equal.
- Timestamps cross the boundary as RFC 3339 and are formatted in the browser
  with `js_sys::Date`, so the viewer sees their own time zone. `shared` depends
  on `serde` only ([Workspace](../design/workspace.md)) and a date type would
  have broken that for both targets.
- The AWS session in this container is expired, so `terraform plan` and
  `terraform apply` for `infra/identity` have not been run. Until the mapping is
  applied, a signed-in build falls back to `Hello, there.` and the placeholder
  avatar, because the claims do not exist yet.

## Verification

Run on 2026-08-08, in the devcontainer:

| Check | Result |
| --- | --- |
| `just fmt-check` | pass |
| `just lint` | pass — clippy for the host and for WASM, warnings denied |
| `just check` | pass — workspace for the host, `app` for `wasm32-unknown-unknown` |
| `just test` | pass — 0 tests, as before |
| `trunk build` | pass |
| `just tf-fmt-check` | pass |
| `terraform validate`, `identity` and `api` | pass |
| `just tf-validate` | fails at `bootstrap`, before reaching any layer this work touches |
| `GET /health` | `ok` |
| `GET /api/greeting` | 404 — the endpoint is gone |
| `GET /api/dashboard` | the expected payload: `total` 50 over ten daily counts, ten records, newest first |

The `just tf-validate` failure is environmental and predates the change: this
working tree has a real `infra/bootstrap/backend.tf` — gitignored, copied from
the example on this machine — so that layer's `init` reaches for AWS credentials
the expired session cannot supply. Reproduced with the change stashed, and the
two layers past it validate individually.

**Not yet performed**, because the devcontainer has no browser: the visual pass
at 320 and 430 pixels wide and at a viewport height of 720 or less, the
keyboard-focus pass, the reduced-motion pass, and the signed-in home and Google
provider hint under `just dev-web-auth`.

## Retirement

- [ ] Design Documents updated
- [ ] Decision Records written (DR-____)
- [ ] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
