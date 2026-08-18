# Implement CRUD for the action record entity

Status: in progress
Started: 2026-08-17
Branch: main

## Request

Implement full CRUD — list, create, read, update, delete — for the action
record entity ("actions": a registered action type paired with a recorded
numeric value), across `crates/shared`, `crates/server` and `crates/app`,
using the four static HTML mockups just added under `docs/design/html/`
(`actions-list.html`, `actions-create.html`, `actions-edit.html`,
`actions-delete-confirm.html`) as the visual and interaction reference,
after reading `docs/README.md`'s documentation model and reviewing those
mockups.

### Clarifications

Asked to weigh in on the one open architectural fork named in Interpretation
below — how `GET`/`PUT`/`DELETE /api/actions/{id}` locates one action record
given the sort key `RECORD#<recorded_at>#<ulid>` — after being shown three
concrete options (keep the key and locate by a partition `Query`; add a GSI
keyed by the bare id; drop `<recorded_at>` from the sort key entirely). The
user confirmed the recommended option: keep the existing key encoding
unchanged and locate a record by querying the owner's `RECORD#` range and
matching the trailing `#<ulid>` before operating on the found key.

## Interpretation

**What is being asked.** Bring the action-record entity to the same
implemented state action types already have: a `Store`-backed list, create,
get, update and delete, exposed as `/api/actions*` and validated the way
`action_types.rs` validates; and three new authenticated screens —
`/actions`, `/actions/new`, `/actions/:id` — replacing the `NotFound`
fallback those paths currently resolve to, built to the four new HTML
references and reusing this project's established patterns (`LocalResource`
+ `Suspense` for reads, `spawn_local` + local `saving`/`deleting` signals for
writes, a native `<dialog>` for delete confirmation, a compact-selector +
searchable-modal-picker pattern for choosing a related entity — DR-0013's
approach, generalized from icons to action types).

**Out of scope, assumed:**
- Connecting `GET /api/dashboard` to the real store. `backend.md` already
  names this a separate, deferred piece of work ("Only the body of that
  handler changes when it is") and doing it here would also pull in the
  dashboard's two window queries — a materially separate change.
- A field for backdating when an action happened. None of the four mockups
  include one; `actions-create.html` has only an action-type selector and a
  value field, so `recorded_at` continues to mean "now, at creation" exactly
  as `persistence.md` already describes it.
- Any business-rule validation of the numeric value beyond it being a finite
  number. No design document states a min, a max, or a per-unit rule, and
  `action_types::validate` sets the precedent of checking structure, not
  domain meaning, for exactly this reason.
- CI, dependency auditing, and the `auth.rs` pure-logic extraction — already
  out of scope per the just-closed test-strategy Work Log, untouched by this
  one.

**Assumptions carried into the plan below, flagged here because they are
naming or reuse calls rather than requirements stated anywhere:**

- Module and route naming mirrors the existing convention
  (`action_types.rs` ↔ `/api/action-types`) rather than the wire type's own
  name: a new `actions.rs` in both `crates/server` and `crates/app`, behind
  `/api/actions` and `/actions*`, even though the shared struct stays
  `ActionRecord`/`NewActionRecord` — `persistence.md` and `page-layouts.md`
  already call the entity an "action record" but the user-facing screens and
  the four mockup filenames all say "action[s]".
- CSS reuses rather than parallels where the shape already matches:
  - The actions list reuses the dashboard's existing `.activity-*` classes
    (identical row shape to the mockup's `.record-*` classes) with the
    action-types index's `.edit-icon` chevron in place of the dashboard's
    `.repeat-icon` plus, since a list row here opens editing, not creation.
  - The delete-confirmation dialog reuses `.confirm-dialog`/`.dialog-*`
    outright, adding one small `.activity-summary` container around the
    existing `.activity-icon`/`.activity-copy`/`.activity-name`/
    `.activity-time`/`.activity-value` classes, the same way the action-type
    version adds only `.type-summary`/`.type-kind` around its own reused
    parts.
  - The create form's action-type picker becomes a new `TypeField` component
    mirroring `IconField`'s shape exactly (compact trigger + native
    `<dialog>` with a search field and a single-select radio group), with
    new CSS for what does not already exist: `.type-select*` for the
    trigger, `.type-dialog*` for the modal shell, and `.result-unit` beside
    the already-existing `.result-icon`/`.result-name`/`.result-check`.
  - New CSS for what neither existing form has: `.value-input-wrap`/
    `.value-unit` for the numeric field's unit suffix, and
    `.type-readonly*`/`.readonly-value` for the edit screen's two read-only
    fields (type, recorded time).
- `type_id` in the create body naming no action type in the caller's own
  partition is answered `400` ("rejected"), not `404` — it is a body field
  referencing another entity, not the URL's own addressed resource, which is
  what `404` means for `action_types`'s `{id}` path parameter.
- An edit changes only `value`. `type_id`, the copied `name`/`unit`/`icon`,
  and `recorded_at` are untouched, per DR-0016 and the edit mockup's own
  copy ("The type is fixed once a record is created (DR-0016)").
- Tests follow `testing.md`'s existing shape exactly: in-module
  `#[cfg(test)] mod tests` in `store.rs` (a `Memory` set plus an `#[ignore]`d
  `Dynamo` subset) and in the new `actions.rs` (`validate`), and new cases
  appended to `main.rs`'s router-level test module. No new `crates/app`
  tests, matching its current zero and `testing.md`'s stated DOM-testing gap.

**One design point was a genuine fork with a real trade-off, not a stated
requirement — flagged rather than decided silently, per this project's own
practice of writing a Decision Record for exactly this shape of choice. Now
resolved by the user; see Clarifications above.**

DynamoDB's key for a record is `sk = RECORD#<recorded_at>#<ulid>`. The API
exposes only the bare ULID as `id` (`persistence.md`'s existing constraint),
but unlike an action type's `sk = TYPE#<ulid>` — where the id alone
reconstructs the key — a record's full sort key cannot be rebuilt from `id`
alone, because `<recorded_at>` sits between the prefix and the ULID.

Recommended approach, carried into the plan below: `GET`/`PUT`/
`DELETE /api/actions/{id}` run the same partition `Query`
(`begins_with(sk, "RECORD#")`) the list endpoint already runs, then locate
the one item whose key's trailing `#<ulid>` matches `id` before issuing the
`GetItem`/`UpdateItem`/`DeleteItem` against its now-known full key. This
needs no schema or index change and stays inside the existing IAM grant
(`Query`, scoped to one partition — never `Scan`), at the cost of one
full-partition read per single-record operation instead of one point read.
That cost is acceptable at this project's personal-habit-tracking scale and
matches `persistence.md`'s own note that a secondary index is deferred
"until a pattern needs it, " which this arguably now does — the choice not
to add one yet is part of what the write-up above should say.

This becomes a Decision Record (see Plan step 12), not a silent
implementation detail: a future GSI would change how every one of these four
handlers is written. Two rejected alternatives are worth keeping in that
record precisely because they will not be preserved anywhere else once this
Work Log is deleted:

- **A GSI keyed by the bare id.** Rejected for now, not permanently — it is
  genuine O(1) point access and is the shape DR-0015 already anticipated
  ("adding an index is an online operation"), but `infra/data/main.tf`
  currently carries an explicit comment committing to no secondary index,
  and adding one here would both reverse that stated position and require
  writing `id` as its own top-level attribute (today it exists only encoded
  inside `sk`), a real write-cost increase for a query pattern this
  project's scale does not yet need.
- **Dropping `<recorded_at>` from the sort key**, making it `RECORD#<ulid>`
  like an action type's `TYPE#<ulid>`. Ordering is unaffected today only
  because `recorded_at` always equals creation time — no backdating field
  exists yet — but this would silently foreclose backdating as a future
  feature and would invalidate `persistence.md`'s already-designed (if not
  yet implemented) dashboard window query,
  `sk BETWEEN "RECORD#<from>" AND "RECORD#<to>"`, which relies on the sort
  key carrying a human-meaningful, lexically-ordered date directly. Reworking
  that query against synthetic ULID-timestamp bounds instead is exactly the
  kind of less-transparent technique `persistence.md` chose the fixed-width
  RFC 3339 format to avoid.

## Plan

1. `crates/shared/src/lib.rs`: add `NewActionRecord { type_id, value }` and
   `UpdateActionRecord { value }`, doc-commented like the existing types.
2. `crates/server/src/store.rs`: turn `Store::Memory` into a struct variant
   holding both a `types` map and a `records` map (mirroring one DynamoDB
   table holding both), updating every existing construction call site.
   Add `list_action_records`, `create_action_record` (looks up the named
   type to copy `name`/`unit`/`icon` and to confirm ownership, mints a ULID
   and a `now()` timestamp), `get_action_record`, `update_action_record`
   (value only) and `delete_action_record`, the latter three using the
   partition-Query-then-locate approach above for `Dynamo` and a direct
   `Vec` search for `Memory`. Extend `dynamo_tests` with an opt-in subset
   mirroring the existing four.
3. `crates/server/src/actions.rs` (new): `list`/`create`/`get_one`/`update`/
   `delete` handlers mirroring `action_types.rs`'s shape, a `validate` that
   checks `type_id` names an owned action type and `value` is finite, and
   its own `Failure` enum with this module's own messages.
4. `crates/server/src/main.rs`: register `/api/actions` and
   `/api/actions/{id}`; extend the router-level tests with a
   create→list→get→update→delete round trip and a bad-`type_id` rejection.
5. `crates/app/src/api.rs`: add `fetch_action_records`, `create_action_record`,
   `fetch_action_record`, `update_action_record`, `delete_action_record`.
6. `crates/app/src/actions.rs` (new): `ActionsPage` (list), `NewActionPage`
   (type picker + value field, reading an optional `?action_type=` query
   parameter to preselect — the dashboard's repeat link already sends one),
   `EditActionPage` (read-only type and recorded time, editable value, and
   the delete flow with its own native confirm `<dialog>`, mirroring
   `EditActionTypePage`'s).
7. `crates/app/src/actions.rs` or a new sibling module: `TypeField`,
   mirroring `IconField`'s compact-selector-plus-modal-picker shape for
   choosing an action type instead of an icon.
8. `crates/app/src/app.rs`: replace the three `/actions*` paths currently
   falling through to `NotFound` with the real routes, each behind
   `RequireAuth`.
9. `style/main.css`: add the rules named in Interpretation above
   (`.type-select*`, `.type-dialog*`, `.result-unit`, `.value-input-wrap`,
   `.value-unit`, `.type-readonly*`, `.readonly-value`, `.activity-summary`),
   plus the small `.activity-link:hover .edit-icon` rule the list needs.
10. Draft updates to `backend.md`, `persistence.md`, `frontend.md` and
    `page-layouts.md` — Structure/Interfaces tables, the query table, the
    "Actions"/"Add action" sections replacing the current
    "Remaining application screens" stub, and the Navigation diagram —
    held for the human confirmation Design Documents need before the work
    is considered complete.
11. Write a Decision Record for the partition-Query-then-locate approach to
    addressing one action record by id, once confirmed.
12. Verify: `cargo test --workspace`, `just test-dynamo`, `just fmt-check`,
    `just check`, `just lint`; exercise the three new screens by hand via
    `just dev-web`/`dev-web-auth` for the golden path and the delete
    confirmation.

## Progress

### 2026-08-17

Read `docs/README.md`, the four new HTML mockups, `docs/design/index.md`
and every Design Document the change touches (`backend.md`, `persistence.md`,
`page-layouts.md`, `frontend.md`, `testing.md`), and the existing action-type
implementation end to end (`shared::lib`, `server::{action_types, store,
main}`, `app::{action_types, api, app, icon_picker}`) and `style/main.css`'s
relevant sections, to match this work to established conventions rather than
inventing new ones. Wrote the Interpretation and Plan above and stopped for
confirmation, per this project's Work Log practice.

Presented the sort-key fork (Interpretation) as three concrete options,
checking `infra/data/main.tf` and `justfile`'s `dynamo-table` recipe first
so the GSI option's cost was stated accurately rather than estimated. The
user confirmed Option 1 (keep the key, locate by partition `Query`) — see
Clarifications. Proceeding with the Plan as written.

Backend done (Plan steps 1–4): `shared::{NewActionRecord, UpdateActionRecord}`;
`Store::Memory` restructured into a `{ types, records }` struct variant (every
existing construction site updated, plus a `#[cfg(test)] Store::memory()`
helper to stop the two-map literal from being repeated); `list_action_records`
(newest first — `scan_index_forward(false)` for `Dynamo`, reversed `Vec` for
`Memory`), `create_action_record` (looks up the type, copies its display
attributes, `None` for an unowned `type_id`), `get_action_record`,
`update_action_record` (value only), `delete_action_record` — the latter
three using `find_action_record`'s confirmed Query-then-match approach for
`Dynamo`. New `server::actions` module mirrors `action_types.rs`'s shape,
with its own `Failure` (a bad `type_id` is `400`, not `404` — it is a body
field, not the URL's addressed resource). Routes registered in `main.rs`.

Verified: `cargo test -p server` — 59 passed, 8 ignored (was 45/4 before this
work); `just test-dynamo` — all 8 opt-in DynamoDB tests pass, including the
new `find_action_record` Query-then-match path against a real DynamoDB
Local, not just `Memory`. `cargo fmt -p server -p shared --check` and `cargo
clippy -p server -p shared --all-targets -- -D warnings` both clean.

Frontend done (Plan steps 5–9): `api.rs` gained the five `/api/actions*`
calls; new `crates/app/src/actions.rs` (`ActionsPage`, `NewActionPage` +
`NewActionForm`, `EditActionPage` + `EditActionForm`) mirrors
`action_types.rs`'s shape throughout; new `crates/app/src/type_picker.rs`
(`TypeField`) mirrors `IconField` exactly for choosing a registered type
instead of an icon, reading the account's own types (loaded once by the page
above it) rather than a compile-time catalog; new `crates/app/src/format.rs`
factors `format_value`/`format_timestamp` out of `dashboard.rs`, which was
their only caller before `actions.rs` became a second one. `app.rs` gained
the three real routes behind `RequireAuth`, replacing the `NotFound`
fallback comments that described them as not yet built. Renamed the
existing `.empty-types` CSS class to `.empty-state` since it is now shared by
three empty conditions, not one, updating its three call sites.

Two implementation issues surfaced and were fixed before any of this
compiled or passed lint, not after:
- `TypeField` first held its shared `types: Vec<ActionType>` in an `Rc`,
  matching a plain first instinct; Leptos's signals require `Send + Sync`
  even in this single-threaded CSR build, which `Rc` does not have and
  `IconField`'s `&'static [Icon]` gets for free — switched to `Arc`.
- `EditActionForm` used a new `"edit-form"` CSS class, following the
  mockup's own per-page naming; `action_types.rs`'s real `EditForm` in this
  app already reuses `"create-form"` for both create and edit, and `main.css`
  only contracts `.create-form` at short viewports. Switched to match the
  existing app-level precedent rather than the standalone mockup's naming.

Verified: `just fmt-check`, `just check` (workspace + `wasm32-unknown-unknown`
target) and `just lint` (same two targets, `-D warnings`) all clean; `trunk
build` succeeds. `just test` — 59 passed, 8 ignored, unchanged from the
backend-only checkpoint. Attempted a live browser check per this project's
own UI-verification norm: started `just dev-api` and `just dev-web`, then
tried Claude in Chrome (not connected in this session) and the `run` skill's
`chromium-cli` fallback (not installed) — neither available, which matches
`testing.md`'s and `workspace.md`'s already-documented constraint that this
devcontainer has neither a browser nor Node.js. Visual verification of the
three new screens is therefore a manual step for the user, not something
this session could complete; both dev servers were left running
(`:3000`/`:8080`) for that purpose. In their place, ran a `curl`-based
end-to-end smoke test against the live `dev-api` (create a type, create a
record against it, get, update, delete, confirm a post-delete `404`) —
passed on the second attempt after a scripting mistake in the first (a
greedy `sed` pattern captured `action_type.id` instead of the record's own
`id`) produced misleading 404s that were the script's fault, not the
server's; re-run with corrected extraction confirmed every step.

Design Documents updated (Plan step 10, drafted and edited directly per this
project's practice — see the 2026-08-16 test-strategy Work Log for the same
approach): `persistence.md` (top note, the query table gained "Actions list"
and "Get / edit / delete an action" rows and a note that the dashboard's two
queries stay unimplemented, two new Constraints bullets), `backend.md`
(`actions.rs` in the Structure table, five new Interfaces rows, the
Validation/Failures prose extended, a new "Locating one action record"
paragraph, the in-memory-store Constraints bullet extended for records'
reversed ordering), `frontend.md` (`actions.rs`/`type_picker.rs`/`format.rs`
in the Structure table, the routing paragraph, the account-menu paragraph's
stale "not built" note removed, a new type-picker paragraph, the Writes and
Deletion paragraphs extended, the Consumes line), `page-layouts.md` (the
"Remaining application screens" stub replaced with full "Actions", "Add
action", "Edit action" and "Delete action confirmation" sections citing the
four HTML references, the Navigation diagram and Interfaces data table
updated, two new Constraints bullets), and `testing.md` (the coverage table's
`server` row, the router-level-tests and opt-in-DynamoDB paragraphs extended
for the new round trip and the `find_action_record` path). `docs/design/index.md`
gained the DR-0032 row and its own date bump.

Wrote [DR-0032](../decisions/DR-0032-an-action-record-is-located-by-owner-partition-query-not-a-secondary-index.md)
for the sort-key fork the Clarifications section above records the user's
answer to, including the two rejected alternatives (a GSI keyed by the bare
id; dropping `recorded_at` from the sort key) and why each was rejected now
rather than permanently.

## Verification

`cargo test -p server` (before the frontend work) and `just test` (after):
59 passed, 0 failed, 8 ignored throughout — 21 new tests over the
pre-existing 45/4 (13 `Memory` tests, 4 opt-in `dynamo_tests`, 3 `validate`
tests, and the store test count includes the new action-record cases listed
in Progress). `just test-dynamo` run once after the backend landed: all 8
opt-in tests pass against a real DynamoDB Local, including
`find_action_record`'s Query-then-match path. `just fmt-check`, `just check`
and `just lint` (both the workspace and the `wasm32-unknown-unknown` `app`
target, `-D warnings` throughout) all clean after every round of changes,
most recently after the Design Document pass (which touched no code). `trunk
build` succeeds. A `curl`-based end-to-end smoke test against a live
`dev-api` confirmed the full create → list → get → update → delete cycle for
an action record, including a `404` on the id after deletion. A live-browser
check of the three new screens was attempted and could not be completed in
this session — see Progress — and remains open for the user.

## Retirement

- [x] Design Documents updated — `persistence.md`, `backend.md`,
      `frontend.md`, `page-layouts.md`, `testing.md` and `docs/design/index.md`
      all reflect the actions CRUD work as implemented. Per this project's
      practice (docs/README.md's Ownership section), an agent may draft these
      updates but a human confirms them before this item is genuinely done —
      that confirmation is the one thing this Work Log cannot itself supply,
      the same open point the 2026-08-16 test-strategy log left for
      `testing.md`.
- [x] Decision Records written — DR-0032, recording the sort-key fork the
      user resolved (Clarifications) and the two alternatives rejected for
      now.
- [x] Non-obvious knowledge preserved — DR-0032 covers the durable reasoning;
      the two implementation pitfalls in Progress (`Rc` vs `Arc` under
      Leptos's `Send + Sync` bound, and the `"edit-form"` vs `"create-form"`
      class-naming mismatch against this app's actual precedent) are
      recorded there because neither has anywhere else to live and both
      would otherwise be relearned by the next person to write a similar
      component.
- [ ] No durable document depends on this log — true once the Design Document
      confirmation above lands; nothing currently cites this log by name.
