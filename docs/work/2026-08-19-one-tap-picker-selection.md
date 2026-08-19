# Selecting a row in the icon and action-type pickers applies it immediately

Status: complete
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

4. ~~No HTML mockup under `docs/design/html/` needs updating: `page-layouts.md`
   cites no mockup file for the icon-picker modal state, and no mockup exists
   for the type picker at all — only `page-layouts.md`, `frontend.md`, and
   the two components' own doc comments describe this interaction today.~~
   **Superseded, see Progress (2026-08-19): this was wrong.** Three mockups
   under `docs/design/html/` do embed the picker's modal markup and its own
   vanilla-JS apply logic — `action-types-create.html`,
   `action-types-edit.html` (both the icon picker) and `actions-create.html`
   (the type picker) — even though `page-layouts.md`'s prose does not link
   them specifically for that state; they were found by grepping the repo
   for the button text and class name this change removes. They are updated
   too, as Plan step 7 below.

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
7. **(Added 2026-08-19, correcting Interpretation assumption 4 above.)**
   Update `docs/design/html/action-types-create.html`,
   `action-types-edit.html` and `actions-create.html`: remove each mockup's
   own `.apply-icon-button`/`.apply-type-button` CSS and button markup, and
   replace the JS `updateApplyState`/`change`-listener/apply-button-click
   logic with a per-option `click` listener that applies and closes
   immediately, mirroring the same click-not-change reasoning as the Rust
   components.

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

User confirmed; proceeding with implementation.

Plan steps 1–3 done: `crates/app/src/icon_picker.rs`'s `IconField` and
`crates/app/src/type_picker.rs`'s `TypeField` both dropped their
`staged`/`apply`/`cannot_apply` machinery in favor of a `select` closure
bound to each result row's `on:click` (not `on:change`, per Interpretation
point 2), which validates the choice, writes the form's real signal
directly, and closes the dialog; `prop:checked` now compares against that
real signal instead of a staged copy. Both components' doc comments were
updated to describe one-tap selection and cite DR-0035 (not yet written —
see step 5) alongside the still-applicable DR-0013. `style/main.css`'s
`.apply-icon-button`/`.apply-type-button` rules (base, `:disabled`, and the
shared `:focus-visible` rule) were removed.

While confirming no other reference to the removed classes remained
(Plan step 3's own check, widened to the whole repo rather than just
`style/main.css`), found that **Interpretation assumption 4 was wrong**:
`docs/design/html/action-types-create.html`, `action-types-edit.html` and
`actions-create.html` all embed the picker's modal markup and its own
vanilla-JS apply logic, unlike what a reading of `page-layouts.md`'s prose
alone suggested. Marked assumption 4 superseded above and added Plan step 7
to cover it, then executed it: all three mockups had their
`.apply-icon-button`/`.apply-type-button` CSS and button markup removed, and
their `updateApplyState`/`change`-listener/apply-button-click JS replaced
with a per-option `click` listener (`applyChoice`) that applies and closes
immediately — the same click-not-change reasoning as the Rust components,
restated in a short inline comment in each mockup rather than a citation,
since these mockups are static references outside the crate and do not cite
Decision Records elsewhere either. Verified via `grep` across
`docs/design/html/*.html` that no reference to the removed button classes,
`data-apply-*` attributes, or `updateApplyState` remains.

Plan step 4 done: drafted directly into `docs/design/frontend.md` (the icon
picker paragraph — the staged-choice/`Use selected icon` sentence replaced
with the one-tap rule and the click-vs-change reasoning, citing DR-0035
narrowing DR-0013; the type-picker paragraph needed no separate edit, since
its prose already deferred to `IconField`'s description rather than
restating it) and `docs/design/page-layouts.md` (the "Action type icon
picker" section's diagram and prose, the "Add action" section's picker
sentence, and both Navigation-diagram `── Use selected * ──▶` lines,
collapsed to `── select a result ──▶`, keeping their matching close/Escape
lines for the no-op case). Per this project's practice, these are drafts
directly edited into the file, held for the user's confirmation before the
work is considered complete (see Retirement).

While re-checking for stray references before moving on, found a second,
narrower miss beyond assumption 4: `docs/design/visual-design.md` also
described the icon picker's now-removed "final `Use selected icon` action"
in its own styling prose (the accent-surface treatment of a button that no
longer exists). This is the same class of gap as assumption 4 — a Design
Document describing DR-0013's UI beyond the two files checked directly
during Interpretation — corrected in place the same way, citing DR-0035. A
repo-wide `grep` across `docs/design/*.md` and `docs/decisions/*.md`
afterward found nothing else, confirming DR-0013 itself (correctly
untouched, since Decision Records are append-only) is the only remaining
mention of the retired staged-apply behavior.

Plan step 5 done: wrote
[DR-0035](../decisions/DR-0035-a-picker-row-applies-on-click-not-on-change.md),
narrowing DR-0013 — its Decision keeps DR-0013's dialog/search/radiogroup/
WAI-ARIA reasoning intact and states only the staged-then-apply claim is
superseded, its Alternatives records the rejected `change`-based approaches
(both the direct one and an auto-activate-the-button variant) with the
arrow-key-browsing reason each was rejected for, and its Consequences names
the still-open manual-browser-verification item so it is not lost once this
Work Log is deleted. Updated DR-0013's Status line to `narrowed by DR-0035`
(the one permitted edit to an existing Decision Record) and
`docs/design/index.md`'s Decision Records table (the DR-0013 row's
annotation, a new DR-0035 row).

Plan step 6 done: `cargo fmt -p app --check`, `just check` (workspace +
`wasm32-unknown-unknown`), `just lint` (same two targets, `-D warnings`),
`trunk build`, and `just test` all clean — 64 passed, 0 failed, 10 ignored,
unchanged from before this work, matching `testing.md`'s documented gap that
`crates/app` has no DOM tests to be affected by a UI-only change. The manual
browser pass itself (mouse/touch tap applies and closes; arrow-key browsing
does not; Space applies and closes) could not be performed in this session —
this devcontainer has no browser, per `testing.md`/`workspace.md` — and
remains open for the user, exactly as flagged in Interpretation point 2 and
DR-0035's Consequences.

### 2026-08-19 (later)

The user confirmed the manual browser pass Plan step 6 and Interpretation
point 2 left open — the one item this session could not check itself, since
the devcontainer has no browser. Marked done below.

The user also reviewed and approved the drafted Design Document changes
(`frontend.md`, `page-layouts.md`, `visual-design.md`,
`docs/design/index.md`) via the diff shown to them, satisfying this
project's Ownership rule that an agent may draft such an update but a human
confirms it before the work is considered complete. Every Retirement item
now holds; closing this Work Log out.

## Verification

`cargo fmt -p app --check`, `just check`, `just lint` (workspace and
`wasm32-unknown-unknown`, `-D warnings`), `trunk build`, and `just test` all
pass — 64 passed, 0 failed, 10 ignored, unchanged from the pre-existing
baseline. A repo-wide `grep` after every edit found no remaining reference
to the removed `.apply-icon-button`/`.apply-type-button` classes,
`data-apply-*` attributes, `updateApplyState`, or the `Use selected icon`/
`Use selected type` button text outside DR-0013 itself (correctly untouched,
per Decision Records being append-only). The user has confirmed the
live-browser pass by hand: a tap/click on a result in both pickers applies
it and closes the dialog with no further tap, matching the request this
work started from.

## Retirement

- [x] Design Documents updated — `frontend.md`, `page-layouts.md`,
      `visual-design.md` and `docs/design/index.md` all reflect one-tap
      picker selection and cite DR-0035. Drafted by the agent and confirmed
      by the user (see Progress, 2026-08-19 later entry), per this project's
      Ownership rule that a human confirms a Design Document overwrite.
- [x] Decision Records written — DR-0035, narrowing DR-0013's Status line.
- [x] Non-obvious knowledge preserved — DR-0035 covers the click-vs-change
      keyboard reasoning and the rejected alternatives; nothing else surfaced
      in this work has no other home.
- [x] No durable document depends on this log — the Design Document
      confirmation landed and the user completed the manual-browser
      verification; nothing cites this log by name.
