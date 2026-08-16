# Action types: editing and deletion

Status: in progress
Started: 2026-08-16
Branch: main

## Request

Implement the remaining CRUD operations for the action-type entity. Listing and
creation already exist end to end; reading a single type, updating one, and
deleting one do not.

## Interpretation

**In scope.** A `GET /api/action-types/{id}` endpoint, an update endpoint, and
a delete endpoint on the service, backed by the store (both the in-memory and
the DynamoDB variants). On the frontend, an edit screen reachable from an
action-type row — currently a dead link caught by the router's `NotFound`
fallback — that loads the type, lets its name/unit/icon be changed and saved,
and offers deletion behind the confirmation dialog `page-layouts.md` and the
two HTML references (`action-types-edit.html`,
`action-types-delete-confirm.html`) already specify.

**Out of scope.** Action records (`RECORD#` items) do not exist yet — the
dashboard still answers from fixed values — so there is nothing to reconcile a
type edit or delete against at runtime. DR-0016 already settled the design
question this would otherwise raise: a record copies its type's display
attributes at write time, so deleting a type is a plain single-item delete with
no cascade, and nothing here needs to touch that design. Also out of scope:
`infra/api` and any other infrastructure layer are declared for these new
methods but not applied — Terraform apply against real AWS is a deploy action
this Work Log does not take on its own.

**Assumptions.**

- **A dedicated `GET /api/action-types/{id}`, not a client-side lookup in the
  already-fetched list.** `persistence.md`'s query table already names `GetItem
  on pk, sk = TYPE#<id>` as the edit screen's query, so this treats that line as
  the intended design rather than a new one. It also means a direct navigation
  to `/action-types/{id}` (a reload, a bookmark) works without depending on the
  index screen having been visited first in the same session.
- **Update is `PUT`, delete is `DELETE`, both under `/api/action-types/{id}`,**
  matching the REST verbs `persistence.md`'s table already implies
  (`UpdateItem`, `DeleteItem`) and axum 0.8's routing.
- **An update is conditioned on the item already existing**
  (`attribute_exists(pk)` in DynamoDB terms), so a `PUT` against an id that is
  not in the caller's partition answers `404` rather than silently creating an
  item at that key. A delete is not conditioned — `DeleteItem` is naturally
  idempotent, and there is no design requirement to distinguish "already gone"
  from "gone now."
- **The confirmation dialog is a native `<dialog>` shown with `showModal`,**
  the same construction `icon_picker::IconField` already uses for DR-0013.
  The HTML reference simulates it with a fixed overlay `<div>` because that
  file is a static mockup with no script driving a real dialog; the built
  screen does not need that layer and gets centering, focus containment and
  Escape-to-close from the browser the same way the icon picker does.
- **Both attribute names besides the two already handled (`unit`, `icon`) are
  aliased in the DynamoDB `UpdateExpression`,** not just `name` — `NAME` is a
  documented reserved word and I did not want to depend on checking whether
  `UNIT` or `ICON` are too.
- **Saving an edit and confirming a delete both return to `/action-types` on
  success,** mirroring what creation already does and what the nav table in
  `page-layouts.md` shows for delete. Both are silent on this specifically, so
  this is inference, not a stated requirement.
- No unverified-external-system risk here worth a spike: the store methods this
  adds are the same three DynamoDB operations (`GetItem`, `UpdateItem` with a
  condition, `DeleteItem`) the SDK already exercises elsewhere in this file for
  `Query`/`PutItem`, and `just dev-api-dynamo` checks the real DynamoDB Local
  path the way the existing tests do.

## Plan

1. **Store** (`crates/server/src/store.rs`): add `get_action_type`,
   `update_action_type` (returns `Option<ActionType>`, `None` on a missing
   item), `delete_action_type`, each with a `Dynamo` and a `Memory` arm. Cover
   with unit tests mirroring the existing `create`/`list` ones — including that
   an edit preserves list order and that update/delete are scoped to the
   caller's own partition.
2. **Handlers** (`crates/server/src/action_types.rs`): add `get_one`, `update`,
   `delete`, reusing `validate` for the update body. Add a `Failure::NotFound`
   variant answering `404` with a plain-word reason, for a missing id on
   `get_one`/`update`.
3. **Router** (`crates/server/src/main.rs`): mount
   `/api/action-types/{id}` with `get`, `put`, `delete`.
4. **Shared/API client** (`crates/app/src/api.rs`): add
   `fetch_action_type`, `update_action_type`, `delete_action_type`, following
   the existing `get_json`/`post_json` shape (a `put_json` and a bodiless
   `delete` helper).
5. **Icon** (`crates/app/src/icons.rs`): add a `Trash` glyph, matching the two
   HTML references.
6. **Edit screen** (`crates/app/src/action_types.rs`): add
   `EditActionTypePage`, reading `id` from the route, loading the type behind
   `Suspense`, and rendering the form (reusing `IconField` and the
   `create-form` styling) plus the danger-zone section and the delete
   confirmation `<dialog>`.
7. **Routing** (`crates/app/src/app.rs`): add the guarded
   `/action-types/:id` route; update the comment that currently lists editing
   as one of the two dead links the `NotFound` fallback still catches.
8. **Styles** (`style/main.css`): add rules for the danger zone, the delete
   trigger, and the confirmation dialog, reusing existing classes
   (`type-name`, `unit-value`, `create-form`, the `icon-dialog` pattern) where
   the mockups' own markup already matches them.
9. **Infrastructure** (`infra/api/apigateway.tf`): add `PUT` and `DELETE` to
   `local.api_methods`. Not applied as part of this work.
10. **Verify**: `just fmt`, `just lint`, `just check`, `just test`; exercise
    the edit and delete flows by hand against `just dev-api` /
    `just dev-web`, and against `just dev-api-dynamo` for the DynamoDB path.

## Progress

### 2026-08-16
Read `docs/README.md`, the durable layer (`backend.md`, `persistence.md`,
`page-layouts.md`, `frontend.md`), the two relevant Decision Records already
cited above, the retrospective, and the existing action-type code on both
sides plus the two HTML references for the edit and delete-confirm screens.
No open Work Log covered this. Wrote the Interpretation and Plan above; not
yet implemented.

Implemented the full plan:

- `store.rs`: `get_action_type`, `update_action_type` (`Option`-returning,
  conditioned on existence for the `Dynamo` arm via
  `attribute_exists(pk)`), `delete_action_type` (unconditioned, idempotent),
  each with a `Dynamo` and a `Memory` arm, plus unit tests for ownership
  isolation, in-place update preserving list order, and idempotent delete.
- `action_types.rs` (server): `get_one`, `update`, `delete` handlers; a
  `Failure::NotFound` variant answering `404` in plain words.
- `main.rs` (server): mounted `/api/action-types/{id}` with `get`/`put`/`delete`.
- `api.rs` (app): `fetch_action_type`, `update_action_type`,
  `delete_action_type`; factored the shared 401/status-check logic out of
  `decode` into `checked` so the bodiless `delete` helper could reuse it
  without trying to JSON-decode an empty `204` body.
- `icons.rs`: added `Trash` and `Checkmark`, matching the two HTML references'
  exact paths rather than reusing the icon picker's `Check` (confirmed by
  reading `action-types-create.html`'s submit button, which uses `Plus` — the
  existing code already made this same per-screen-fidelity choice).
- `action_types.rs` (app): `EditActionTypePage` (loads by id from the route via
  `use_params_map`) and `EditForm` (the save form, reusing `IconField` and the
  `create-form` styling, plus the danger zone and a native `<dialog>`
  confirmation, shown with `showModal` the same way `IconField`'s picker is).
- `app.rs`: added the guarded `/action-types/:id` route; updated the two
  comments that referenced editing as one of the dead links `NotFound` still
  catches — only recording an action still is.
- `style/main.css`: added the danger zone, delete trigger, and confirmation
  dialog rules, reusing `type-name`, `unit-value`, `create-form` and the
  `icon-dialog` pattern where the mockups' own markup already matched them;
  extended the short-viewport media query the same way the two HTML
  references do.
- `infra/api/apigateway.tf`: added `PUT` and `DELETE` to `local.api_methods`.
  Not applied.
- Also corrected a stale module doc comment in `api.rs` that still described
  the API Gateway JWT authorizer DR-0028 removed, while editing the same file
  for this work — noted here since it is an incidental fix, not part of the
  plan.

Verified: `just fmt`, `just lint`, `just check`, `just test` (38 server tests,
all passing, including the 8 new ones) all pass; `terraform validate` and
`terraform fmt -check` pass for the `api` layer against the one-line
`apigateway.tf` change. Restarted `just dev-api` and exercised the full
lifecycle by hand with `curl`: create → get → get from a different mock
owner (404) → update → get-after-update → update with an invalid body (400,
validation still applies) → update a nonexistent id (404, confirmed it does
not create one) → delete (204) → get-after-delete (404) → delete again (204,
idempotent) → list (empty). All matched the design.

**No browser was available in this session** (no Claude in Chrome connection,
no headless browser binary in the devcontainer) to click through the actual
edit and delete screens. `trunk build`/`trunk serve` succeeded and the SPA
fallback correctly serves `index.html` for `/action-types/<id>` (checked with
curl), and the wasm32 target type-checks and lints clean, but the UI itself —
the form pre-filling correctly, the icon picker opening with the current icon
selected, the confirmation dialog's focus and Escape behaviour, the
error-message states — has not been visually verified. This should be checked
by hand before considering the work fully done.

## Verification

- `just fmt && just lint && just check && just test` — all pass.
- `terraform fmt -check` and `terraform validate` for `infra/api` — pass.
- Manual `curl` exercise of every new endpoint and its edge cases (ownership
  isolation, validation, existence conditioning, idempotent delete) against
  `just dev-api` — matches the design in every case tried.
- **Not done:** visual/interactive verification of the edit and delete-confirm
  screens in a browser. No browser tool was available this session.

## Retirement

- [x] Design Documents updated — `backend.md`'s endpoint table and Failures/
      Validation prose, `persistence.md`'s query table, `frontend.md`'s route
      list, structure table, data-fetching prose and Interfaces line
- [ ] Decision Records written (DR-____) — none needed; every choice made
      (dedicated `GET .../{id}`, `PUT`/`DELETE` verbs, existence-conditioned
      update, unconditioned idempotent delete, native `<dialog>` for the
      confirmation) was already implied by the existing Design Documents and
      DR-0013/DR-0016, not a new decision
- [x] Non-obvious knowledge preserved — the two points worth keeping (the
      `attribute_exists(pk)` update condition, and why the confirm dialog
      needs no `.dialog-layer` div the way the static mockup does) are already
      captured as doc comments in the code itself, which is where a future
      reader of `store.rs`/`action_types.rs` will be
- [ ] No durable document depends on this log — holds once the browser
      verification above is done and this log is deleted
