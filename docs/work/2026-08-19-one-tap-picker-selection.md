# Selecting a row in the icon and action-type pickers applies it immediately

Status: in progress
Started: 2026-08-19
Branch: main

## Request

Remove the extra confirm-button tap from the two searchable modal pickers —
the action-type icon field and the action-type field on the add-action
form — so that choosing a result completes the selection in one action
instead of two: tapping a row should both select it and close the dialog,
rather than requiring a further tap on "Use selected icon" / "Use selected
type" afterward.

## Interpretation

**What is being asked.** Change the interaction of `IconField`
(`crates/app/src/icon_picker.rs`) and `TypeField`
(`crates/app/src/type_picker.rs`) so that activating a result row applies
that choice to the form's own signal and closes the `<dialog>` in the same
action, eliminating the separate "Use selected icon" / "Use selected type"
button both components currently require. The two components are built to
mirror each other exactly (`frontend.md` states this explicitly), so the
change is symmetric across both.

**Out of scope.** No other picker exists in this application. The two
delete-confirmation dialogs use a different pattern (two named buttons, no
staged value) and are untouched. Search filtering, the empty-state message,
and the live result count are untouched. Dismissing the dialog *without*
selecting anything — Escape or the close control — continues to leave the
form's value untouched; that path never wrote to the real signal before and
does not need to now either.

**Assumptions:**

1. This reverses part of an accepted, cited decision —
   [DR-0013](../decisions/DR-0013-action-type-icons-use-a-searchable-modal-picker.md):
   "Selection is staged until the user chooses `Use selected icon`; closing
   or pressing Escape preserves the former value." Both components' doc
   comments and `page-layouts.md`/`frontend.md`'s prose cite DR-0013 for
   exactly this staged-then-apply behavior, so this is a decision reversal,
   not an implementation detail — it needs a new Decision Record narrowing
   DR-0013, not a silent code change. Most of DR-0013's reasoning (native
   `<dialog>`, the search field, the radiogroup, the WAI-ARIA citations)
   still holds; only the staged-with-explicit-apply claim is superseded.

2. **The apply must bind to the row's `click`, not its `change`, event —
   flagged because this rests on an assumption about browser behavior this
   devcontainer cannot check (no browser is available here, per
   `testing.md`/`workspace.md` and the actions-crud Work Log's own note).**
   A native radio group fires `change` (and `input`) on the newly-selected
   radio as arrow-key focus moves between options, with no click involved —
   that is how native radiogroups let arrow keys browse. Binding
   immediate-apply-and-close to `on:change` would therefore close the dialog
   on the very first arrow press, before a keyboard user could see a second
   option. Binding to `on:click` instead fires for a mouse or touch tap
   (including a tap anywhere in the row's `<label>`, which already
   translates to a click on its `<input>`) and for Space-activation of a
   focused radio, but not for arrow-key traversal alone — letting a keyboard
   user browse with arrow keys and commit with Space, while a pointer user
   gets the requested one-tap selection. This needs confirming by hand in an
   actual browser before the work is considered done, the same way the
   actions-crud Work Log left live-browser verification of its own new
   screens open for the user.

3. Once a click applies the choice directly, the `staged` signal and the
   `apply`/`cannot_apply` logic in both components have no further purpose —
   `checked` can compare directly against the real form signal instead of a
   staged copy. This is a natural simplification following from the
   interaction change, not a separate requirement, and removes roughly
   15–20 lines from each of the two already-near-identical components.

4. No HTML mockup under `docs/design/html/` needs updating: `page-layouts.md`
   cites no mockup file for the icon-picker modal state, and no mockup exists
   for the type picker at all — only `page-layouts.md`, `frontend.md`, and
   the two components' own doc comments describe this interaction today.

5. `.apply-icon-button` and `.apply-type-button` in `style/main.css` (the
   base rule, the `:disabled` rule, and the `:focus-visible` rule shared with
   the dialog close buttons) become dead once both buttons are removed, and
   should be deleted rather than left orphaned.

## Plan

1. `crates/app/src/icon_picker.rs`: remove `staged` and `cannot_apply`;
   change the radio's `on:change` (which only staged a value) to `on:click`
   that validates against `icon_catalog::find` and, if known, writes `icon`
   directly and calls `close()`; drop the `apply` closure and the "Use
   selected icon" button; switch `prop:checked` to compare against
   `icon.get()` directly; update the module-level and `IconField` doc
   comments, which currently describe the staged-then-apply rule and cite
   DR-0013 for it, to describe one-tap selection and cite the new DR
   (step 5).
2. `crates/app/src/type_picker.rs`: mirror the same change in `TypeField`
   (`type_id`, validated against `types` by id) — same restructuring, same
   doc-comment update.
3. `style/main.css`: remove the `.apply-icon-button`/`.apply-type-button`
   rules (base, `:disabled`, and the shared `:focus-visible` rule — checking
   for any other reference first).
4. Draft updates to `docs/design/frontend.md` (the icon-picker and
   type-picker paragraphs, which currently describe staging and an explicit
   apply step) and `docs/design/page-layouts.md` (the "Action type icon
   picker" section's prose, the "Add action" section's picker description,
   and the four affected Navigation-diagram lines — the
   `── Use selected icon ──▶` / `── Use selected type ──▶` and their
   matching `── close or Escape ──▶` lines collapse into one
   `── select a result ──▶ the form, applied` line each, with the
   close/Escape line kept for the no-op case) — held for user confirmation
   per this project's Design Document practice.
5. Write a Decision Record narrowing DR-0013: keep its modal/native-dialog/
   search/radiogroup reasoning, replace only the staged-then-apply claim with
   immediate apply-on-select, and record the `on:click`-vs-`on:change`
   reasoning from Interpretation point 2 as a rejected alternative — this is
   exactly the kind of non-obvious constraint that would otherwise vanish
   once this Work Log is deleted.
6. Verify: `cargo fmt -p app --check`, `just check` and `just lint` against
   both the workspace and the `wasm32-unknown-unknown` target, `trunk build`.
   Then a manual browser pass, called out to the user as a required step this
   devcontainer cannot perform itself: open each picker and confirm (a) a
   mouse/touch tap on a result both applies it and closes the dialog with no
   further tap, (b) arrow-key browsing between results does not itself close
   the dialog, and (c) Space on a focused result applies and closes it the
   same as a click.

## Progress

### 2026-08-19

Read `docs/README.md`, the open Work Logs' index (none cover this request),
[DR-0013](../decisions/DR-0013-action-type-icons-use-a-searchable-modal-picker.md),
`frontend.md` and `page-layouts.md`'s picker-related sections, and both
`crates/app/src/icon_picker.rs` and `crates/app/src/type_picker.rs` in full,
to confirm the two components really do mirror each other exactly and to
locate the `staged`/`apply`/`cannot_apply` logic this change removes.
Confirmed via `grep` that `style/main.css` has no other reference to
`.apply-icon-button`/`.apply-type-button` beyond the three rules named in
Interpretation point 5. Wrote the Interpretation and Plan above and stopped
for confirmation, per this project's Work Log practice.

## Verification

Not yet started.

## Retirement

- [ ] Design Documents updated
- [ ] Decision Records written (DR-____)
- [ ] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
