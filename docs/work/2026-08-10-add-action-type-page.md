# Add action type page

Status: in progress
Started: 2026-08-10
Branch: main

## Request

Implement the add action type page — the authenticated, single-purpose form that
Page Layouts specifies under "Add action type". Where the work turns out to be
large, break it into tasks of a reasonable size rather than treating it as one
undivided change.

### Clarifications

The page is to be implemented as a full vertical slice: the screen, the icon
picker, the `POST` endpoint, and the write into DynamoDB — including deriving the
owner from the Cognito subject. Creating a type must actually persist it.

A minimal action types list at `/action-types` is in scope as well, because it is
the destination `Cancel` and a successful creation both navigate to, and because
the dashboard's account control already links there.

The icon catalog is to be limited to a small set of `lucide-leptos` category
features rather than `all-icons`, to keep the wasm bundle down.

## Interpretation

**What is being asked.** One working path from an empty form to a stored action
type, in the shape the durable documents already describe:

- Route `/action-types/new` behind `RequireAuth`, laid out as
  `docs/design/html/action-types-create.html` shows: name, unit, and a compact
  icon selector on one content surface, a solid accent `Create action type`, and
  `Cancel`.
- The icon selector opens the searchable modal picker of DR-0013, rendering
  Lucide geometry locally through `lucide-leptos` and storing the canonical
  kebab-case name (DR-0014).
- `POST /api/action-types` in `crates/server`, writing a `TYPE#<ulid>` item into
  the table exactly as `docs/design/persistence.md` describes, under
  `USER#<cognito sub>`.
- A minimal `/action-types` list, backed by `GET /api/action-types`, so the
  screen's two exits lead somewhere real.

**Out of scope.**

- Editing and deleting an action type, and their confirmation dialog. They share
  the form composition, but they are separate screens with their own references.
- The add action screen and the actions list. The dashboard keeps answering from
  hardcoded values; nothing on it starts reading the table in this work.
- Any infrastructure change. `POST /api/{proxy+}` is already routed behind the
  authorizer and `POST` is already in `local.api_methods` and therefore in the
  CORS configuration, and the Lambda's inline policy already grants `PutItem` and
  `Query` on the table.
- The list screen's loading, error, and pagination behaviour beyond what is
  needed to see created types. Page Layouts leaves those unspecified for that
  screen and says they must not be inferred.

**Assumptions.**

- This stays one Work Log. The five tasks below are one vertical slice of one
  screen; splitting them across logs would separate the endpoint from the only
  caller it has.
- The list screen needs an empty state that Page Layouts does not define, since
  a new account has no types and that is the state creation is reached from. A
  minimal one is written here and offered to Page Layouts at retirement rather
  than inferred silently from another screen.
- Under the Lambda Web Adapter the service reads the caller's `sub` from the API
  Gateway request context the adapter forwards as a header, not from the token,
  which it never sees a signature for. Locally that header is absent, so a
  development identity stands in. This is the first time the service has an
  identity at all, and it is expected to need a Decision Record.
- A ULID is generated in the service, not accepted from the client.
- `name` and `unit` are trimmed and length-limited, and rejected when empty. The
  exact limits are chosen in task 4 and stated there; no document fixes them.
- The stored `icon` is validated against the generated catalog server-side, so
  the constraint in `persistence.md` holds without trusting the client.

## Plan

### 1. The icon catalog

Add `lucide-leptos` 3.26.0 to the workspace, with a small set of category
features, and generate the table that maps a canonical kebab-case name to its
component and its display name.

- `crates/icongen`, a native binary in the workspace, reads the pinned crate's
  `src/lib.rs` from the cargo registry source cache, parses each
  `#[cfg(...)] mod <name>;` gate, keeps the modules at least one enabled category
  admits, and emits `crates/app/src/icon_catalog.rs`: a sorted catalog of
  `(kebab name, display name)` and a `match` from name to glyph. It is run by
  hand when the pin moves, which is what keeps the table in step with the crate
  (DR-0014).
- Replace the ten hand-drawn glyphs in `crates/app/src/icons.rs` with a lookup
  into that table, keeping the generic fallback for an unknown name.
- The dummy dashboard records in `crates/server` carry ids like `running` and
  `water`, which are not Lucide names and would every one of them fall back.
  Rename them to catalog names in the same change.
- Add the `lucide-leptos` license notice to `THIRD_PARTY_NOTICES.md`, which its
  closing paragraph already asks for.
- Measure `trunk build --release` output before and after, and record both.

### 2. The icon picker

`crates/app/src/icon_picker.rs`: the modal of DR-0013 and DR-0014.

Native `<dialog>` driven through `web-sys` (`show_modal`, `close`), a search
input focused on open, a live result count, a scrollable single-select radio
group of catalog rows, staged selection applied only by `Use selected icon`, and
Escape or close leaving the form's value alone and returning focus to the
compact selector. Port the picker's rules from the reference stylesheet into
`style/main.css`.

### 3. The create screen

`crates/app/src/action_types.rs`: `/action-types/new`, wrapped in `RequireAuth`,
composed as the reference shows, with the compact icon selector whose accessible
name carries the current Lucide name. Client-side validation mirrors what the
service enforces and is a convenience, not the check. Port the form's styles.

At this point the form is complete and submits nothing; tasks 4 and 5 give it
somewhere to submit to and somewhere to return to.

### 4. The service learns to write

- `crates/shared`: the request body for creation.
- `crates/server`: `aws-config` and `aws-sdk-dynamodb`, the table name from
  `TABLE_NAME`, one ULID per new type, `PutItem` with the attributes
  `persistence.md` lists, validation with a `400` for bad input.
- The caller's `sub`, read from the forwarded request context, with a
  development identity when the header is absent. Note the Decision Record.
- `crates/app/src/api.rs` grows a `post_json` beside `get_json`, inheriting the
  same 401 handling.

### 5. The list, and the two exits

- `GET /api/action-types`: `Query` on `pk = USER#<sub>` with
  `begins_with(sk, "TYPE#")`, returning bare ULIDs as ids.
- `/action-types`: the populated state from
  `docs/design/html/action-types-list.html`, plus a minimal empty state.
- `Cancel` and a successful creation both navigate there.

### 6. Retirement

Draft the Design Document updates — `frontend.md` for the new modules, routes and
dependencies, `persistence.md` for the note that says nothing reads the table
yet, `workspace.md` for `crates/icongen`, `page-layouts.md` for the list's empty
state, `index.md` for whether the backend now warrants its own document — and
have them confirmed before the work is called complete.

## Progress

### 2026-08-10

Wrote the plan above. Two things found while reading, before any code:

**No infrastructure change is needed.** `infra/api/apigateway.tf` routes
`POST /api/{proxy+}` behind the JWT authorizer already, `POST` is in
`local.api_methods` and therefore in `cors_configuration`, and `infra/api/iam.tf`
grants `PutItem` and `Query` on the table. The endpoint is reachable the moment
the code exists.

**Lucide's categories are not shaped like this application's use case**, which
makes "enable a few categories" a weaker lever than DR-0014 assumed. The eight
icons in the create-page reference alone span eight different categories:

| Icon | Category |
| --- | --- |
| `person-standing` | `accessibility`, `people` |
| `droplets` | `weather` |
| `book-open` | `text`, `development`, `gaming` |
| `timer` | `time` |
| `bike` | `transportation` |
| `dumbbell` | `navigation`, `sports` |
| `graduation-cap` | `buildings` |
| `footprints` | `navigation` |

So a catalog that merely reproduces the design reference is already eight of the
crate's forty-one categories, and a catalog worth searching is more. The set task
1 starts from is `people`, `weather`, `text`, `time`, `transportation`, `sports`,
`buildings`, `navigation`, `nature`, `animals`, `food-beverage`, `medical`,
`home`, `travel`.

There may be less to this than DR-0014 feared, though: the features gate whether
a module compiles, and an enabled icon our generated `match` never names is an
unreferenced item in a dependency. A release build should drop it. Task 1
measures the bundle rather than assuming either way, and the answer decides
whether the category list stays as it is, or whether generating only a curated
catalog — DR-0014's stated fallback — becomes the right shape after all. Either
outcome refines what DR-0014 says about this cost and is worth recording.

**Task 1 landed, and the measurement it was there to take is decisive.**

`crates/icongen` reads `crates/app/Cargo.toml` for the enabled categories,
`Cargo.lock` for the pinned version, and the crate's own `lib.rs` out of the
registry, and emits `crates/shared/src/icon_names.rs` and
`crates/app/src/icon_catalog.rs`. It has no dependencies. Two corrections to the
naive module-name conversion turned out to be needed: Lucide writes `3d` and
`2x2` as single tokens, so `axis_3_d` and `grid_2_x_2` have to come back as
`axis-3d` and `grid-2x2` rather than `axis-3-d` and `grid-2-x-2`, which are not
names Lucide has.

The 14 categories admit **725 icons**. Rendering them as `lucide-leptos`
components cost **+1.69 MB of raw wasm, 3.3× the whole bundle**. The hope
recorded above — that unreferenced icons would be eliminated — was wrong, and
wrong for a structural reason worth keeping: `CATALOG` is a static array the
picker walks in full, so every icon is reachable by construction. Category
narrowing is the only lever that shape offers, and it is blunt.

The cost was not the geometry. The 725 icons are **143 KB of SVG children**
between them; the rest was 725 copies of a Leptos component carrying five
reactive props and a derived signal, none of which this application varies.

So the catalog stores the geometry and leaves the components behind, which is
the fallback DR-0014 named for exactly this case. `icongen` extracts the
children of each `<svg>`, `crates/app/src/icons.rs` writes the wrapper once, and
`lucide-leptos` moved off `crates/app` onto `crates/icongen` — where no feature
of it is enabled, so it compiles to an empty library and exists only to pin the
version and put the source in the registry.

| release wasm | raw | gzip -9 |
| --- | --- | --- |
| before the catalog | 721,266 | — |
| catalog as components | 2,406,914 | 456,048 |
| catalog as geometry | 892,102 | 311,474 |

**725 searchable icons for +171 KB of raw wasm.** This reverses part of DR-0014
and needs a Decision Record.

The cost that moved rather than vanished: `cargo check --workspace` now compiles
Leptos for the host once, ~21 s, because `crates/icongen` depends on a crate
that depends on Leptos. That buys the lockfile pin, and it is cached after the
first build.

**Tasks 3 and 5 were done together, and not in the order planned.** The plan put
the screen in task 3 and everything it talks to in tasks 4 and 5. Building the
form against an API function that did not exist yet would have meant writing the
submit path twice, so the frontend was finished in one pass — both screens, both
routes, `shared::NewActionType`, and the two `api.rs` functions — and task 4
became the whole of the service. Task 5 is therefore empty of its own work.

One reuse fell out of it: the account control at the end of the top row was
inline in `dashboard.rs` and is now `app::AccountControl`, since three screens
carry it.

**Task 4 — the service.** `crates/server` grew from one file to five:
`identity.rs`, `store.rs`, `action_types.rs`, `dashboard.rs`, and a `main.rs`
that is now only the router and the startup choice.

Two things here have durable consequences and are recorded rather than left in
the code. The caller's `sub` comes from the `x-amzn-request-context` header the
Lambda Web Adapter forwards — the service never sees the invocation event and
never validates a token — which is **DR-0017**. And `TABLE_NAME` being unset
selects an in-memory store, while a missing request context means a constant
development owner, so `just dev-api` needs no credentials and no configuration;
that is **DR-0018**.

The store is an enum with two variants rather than a trait with two
implementations. There are exactly two, the choice is settled at startup, and
keeping them concrete avoids making every method dyn-safe for a question that is
never asked at runtime.

Rejected an idea worth recording: making the development owner come from an
environment variable, so that production would fail closed if the header ever
went missing. It is in DR-0018's alternatives. The case it guards against does
not occur, and the price is a variable every developer must set — which is the
configuration the decision exists to avoid.

**Task 6 — the durable layer.** `docs/design/backend.md` is new: the index said
a document belonged there "once the service does something", and it does.
`frontend.md`, `persistence.md`, `workspace.md`, `page-layouts.md` and
`index.md` are updated, and `DR-0019` records the icon-catalog measurement
above.

**Asked afterwards whether the icons are what makes the bundle heavy.** Measured
by regenerating the catalog from one category and rebuilding, with every other
line of the feature in place:

| release wasm | raw | gzip -9 |
| --- | --- | --- |
| before this work | 721,266 | — |
| the feature, 5 icons | 924,753 | 326,542 |
| the feature, 725 icons | 1,099,032 | 377,392 |

So the catalog is **174 KB raw and 51 KB compressed — 16% and 13%** of the
bundle. The screens and the picker's own code added more than the 725 icons did,
and the 721 KB that predates all of it is the larger share of both. The answer
is no.

The lever that would matter is a size-tuned `[profile.release]`, and trying it
found a constraint worth keeping: `opt-level = "z"` makes rustc emit
`memory.copy`, and trunk invokes its bundled wasm-opt as `wasm-opt -O` with no
`--enable-bulk-memory`, so the build fails in the wasm-opt step with a validator
error. The current profile only passes because it does not emit those
instructions. Changing the profile therefore means changing how trunk calls
wasm-opt, which is its own piece of work and not this one. Nothing was changed;
`Cargo.toml` carries no `[profile.release]`.

**Deployed, and the API does not start — for a reason outside this work.** Both
artefacts were pushed with `just deploy-api` and `just deploy-web`, and every
API call answers 500. The service is not implicated: the packaged binary now
requires glibc 2.38 and 2.39 symbols that `provided.al2023` does not have, which
this work introduced only by adding the AWS SDK to a build path that was already
unsound. It is a packaging question, it is being answered separately, and its
record is `2026-08-10-api-artefact-packaging.md`.

Nothing in this log's plan changes because of it. What changes is that the
vertical slice is verified locally and not yet in production; see Verification
below.

## Verification

`just fmt-check`, `just lint` and `just check` are clean. `just test` passes 12
tests, all new: the identity header parsed and absent, validation trimming and
refusing, fixed-width timestamps, the `TYPE#` prefix stripped from an id and a
`RECORD#` item declined, and the memory store keeping creation order per owner.

Against `just dev-api` with the in-memory store, over HTTP:

| Checked | Result |
| --- | --- |
| `GET /api/action-types` on a fresh store | `[]` |
| `POST` with `"  Running "` / `" km "` | `201`, stored trimmed |
| `GET` after two creations | both, in creation order |
| `POST` with `icon: "not-a-lucide-icon"` | `400` `That icon is not one of the supported icons.` |
| `POST` with a name of spaces | `400` `An action name is required.` |
| `POST` with a 19-character unit | `400` `A numeric unit can be at most 16 characters.` |
| The same calls under a request-context header naming `cognito-sub-abc` | a separate, empty partition; the development owner's items unchanged |

Against `just dev-web`: the bundle builds, `/action-types/new` is served by
`index.html` as a deep link, and `/api/action-types` through the trunk proxy
reaches the axum server.

`just build` produces a release bundle of 1,099,032 bytes of wasm, 377,392
gzipped — against 721,266 before this work. The picker's 725 rows of markup are
most of what the screens added on top of the catalog itself.

**Not verified: anything against the deployed API.** The function does not
start, for the packaging reason recorded above, so nothing in this slice has
been exercised against the real table, a real Cognito subject, or the API
Gateway authorizer. Everything in the tables above was checked locally.

**Not verified: anything that needs a browser.** The devcontainer has none, as
`workspace.md` already records. The dialog opening and returning focus, the
search filtering, Escape discarding a staged choice, the disabled apply
control, and every question of how the screens look are unchecked. They need
`just dev-api` and `just dev-web` and the forwarded port opened from outside
the container.

## Retirement

- [x] Design Documents updated — new `backend.md`; `frontend.md`,
      `persistence.md`, `workspace.md`, `page-layouts.md` and `index.md`
      revised. **Awaiting confirmation**, since an overwrite is not an agent's
      to make alone.
- [x] Decision Records written — DR-0017, DR-0018, DR-0019
- [x] Non-obvious knowledge preserved — the bundle measurement and why category
      narrowing cannot work (DR-0019), the adapter header the identity depends
      on and what breaks if it moves (DR-0017), the environment-variable
      development owner that was considered and rejected (DR-0018), and the two
      Lucide name conversions the generator has to correct (`3d`, `2x2`), which
      live in `crates/icongen` beside the code that does them
- [ ] No durable document depends on this log

Not done, and deliberately outside this work: editing and deleting an action
type, the dashboard reading the table, and any browser verification.
