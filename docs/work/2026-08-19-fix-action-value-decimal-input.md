# Fix decimal values rejected by the action Value input

Status: in progress
Started: 2026-08-19
Branch: main

## Request

Fix a bug where the Value field on the action create and edit screens rejects
decimal points: a fractional value cannot be entered at all.

## Interpretation

**What is being asked.** Repair the Value input on `/actions/new` and
`/actions/:id` (edit) so a user can type a value containing a decimal point
(e.g. `5.2`) and have it stick and persist. Today the decimal point cannot be
entered.

**Out of scope, assumed:**
- Backend/shared validation. `NewActionRecord`/`UpdateActionRecord::value` is
  already `f64`, and `server::actions::validate_value` already accepts any
  finite value, fractional or not — confirmed by the prior actions-crud Work
  Log's curl smoke test. No backend change is needed.
- Any new numeric formatting or localization behaviour (comma decimal
  separators, thousands separators). Not reported, not implied by the request.
- The four HTML mockups under `docs/design/html/`. They are static references
  without Leptos's reactive value-binding, so they do not exhibit this bug
  themselves and are not being changed.

**Root cause, found while investigating — stated here because it is the
load-bearing finding for the plan below:**

Both Value inputs (`crates/app/src/actions.rs`, `NewActionForm` and
`EditActionForm`) are `<input type="number" ... prop:value=value_text
on:input=...>` — a Leptos-controlled input that re-applies the `value_text`
signal to the DOM node's `value` property on every keystroke.

Per the HTML spec, a `type="number"` input's `value` IDL property returns the
empty string whenever the box's current text does not match the "valid
floating-point number" grammar, and a bare trailing decimal point (`"5."`)
does not match that grammar — digits are required after the `.`. So the
instant a user types the `.`, `event_target_value` reads back a value with
the `.` already stripped, the signal updates to that shorter string, and the
reactive `prop:value` binding immediately writes it back to the DOM, visibly
erasing the `.` the user just typed — before a fractional digit can ever
follow it. This makes decimal entry impossible regardless of typing order,
and is unrelated to keyboard layout or locale.

**The fix:** change both inputs' `type` from `"number"` to `"text"`, and drop
the now-meaningless `step="any"` attribute. Keep `inputmode="decimal"` so
mobile devices still present a numeric/decimal keypad. The existing
`.trim().parse::<f64>()` client-side check and its `"A numeric value is
required."` error message already validate whatever text is entered, so no
other logic changes.

This same interaction would recur on any future `type="number"` input added
to this codebase using the identical `prop:value` + `on:input` controlled
pattern. No current Design Document states a constraint on input `type`
choice, so if the user agrees this is worth a Decision Record, so the
reasoning survives once this Work Log is deleted — flagged here, not decided.
No other numeric input exists in the app today, so nothing else needs
changing as part of this fix.

## Plan

1. `crates/app/src/actions.rs`: in `NewActionForm`'s Value input, change
   `type="number"` to `type="text"` and remove `step="any"`; keep
   `inputmode="decimal"`, `placeholder`, and every other attribute unchanged.
2. `crates/app/src/actions.rs`: apply the same change to `EditActionForm`'s
   Value input.
3. Run `cargo fmt -p app --check`, `just check`, `just lint` (workspace and
   the `wasm32-unknown-unknown` target) to confirm no regressions; run `just
   test` to confirm the test count is unchanged, since no Rust logic changes,
   only two HTML attributes.
4. Verify by hand: start `just dev-api` and `just dev-web`(`-auth`) and, if a
   browser is reachable this session, type a decimal value such as `5.2` into
   both the create and edit Value fields, confirm the decimal point survives
   while typing and the saved record shows the fractional value. If no
   browser is reachable in this devcontainer — as the prior actions-crud Work
   Log found — say so and leave it open for the user, the same way that log
   did.
5. Ask the user whether the root-cause explanation above warrants a Decision
   Record before closing this Work Log, since it is non-obvious and would
   otherwise be lost.

## Progress

### 2026-08-19

Read `docs/README.md`, `docs/design/index.md`, `frontend.md` and
`page-layouts.md` (no documented constraint governs the Value input's HTML
`type`), and the prior `2026-08-17-actions-crud.md` Work Log for context on
how the field was built. Located both Value inputs in
`crates/app/src/actions.rs` and traced the root cause described above.
Checked `style/main.css` for any rule depending on `type="number"` (e.g.
spinner-hiding `appearance`/`::-webkit-inner-spin-button` rules) — none
exists, so the type change carries no CSS side effect. Wrote the
Interpretation and Plan above and stopped for confirmation, per this
project's Work Log practice.

User confirmed the plan and chose to defer the Decision Record question to
`/work-done`'s candidate-list step rather than deciding now.

Applied Plan steps 1–2: in `crates/app/src/actions.rs`, changed `type="number"`
to `type="text"` and removed `step="any"` on both `NewActionForm`'s and
`EditActionForm`'s Value inputs. No other attributes touched; `inputmode="decimal"`
kept on both.

Verified (Plan step 3): `cargo fmt -p app --check` clean; `just check`
(workspace + `wasm32-unknown-unknown`) clean; `just lint` (same two targets,
`-D warnings`) clean; `just test` — 64 passed, 0 failed, 10 ignored, matching
the pre-change count exactly, confirming no Rust logic changed. `trunk build`
succeeds.

Attempted live-browser verification (Plan step 4): no Chrome/Chromium binary
exists in this devcontainer and no `claude-in-chrome` MCP connection is
available in this session — the same constraint the prior actions-crud Work
Log recorded. Live confirmation that the decimal point now survives typing is
therefore left open for the user, same as that log left its own browser check
open.

## Verification

`cargo fmt -p app --check`, `just check` and `just lint` (workspace and the
`wasm32-unknown-unknown` `app` target, `-D warnings` throughout) all clean.
`just test`: 64 passed, 0 failed, 10 ignored — identical to the count before
this change, as expected since only two HTML attributes changed and no Rust
logic did. `trunk build` succeeds. Live-browser confirmation that typing
`5.2` into the create and edit Value fields now keeps the decimal point and
saves the fractional value could not be completed in this session — no
browser is reachable in this devcontainer — and remains open for the user.

## Retirement

- [ ] Design Documents updated — none expected; no Design Document states a
      constraint on the Value input's HTML `type`, and `page-layouts.md`
      already describes the field only as "a numeric value" (unchanged by
      this fix). Confirm no wording implies `type="number"` before closing.
- [ ] Decision Records written (DR-____) — open question for `/work-done`'s
      candidate list: whether the root-cause explanation above (a
      `type="number"` input controlled via `prop:value` cannot accept a
      decimal point, because the HTML value-sanitization algorithm rejects a
      bare trailing `.` and the reactive binding immediately echoes that back
      to the DOM) is worth preserving so it is not silently relearned if a
      future numeric input reintroduces the same pattern.
- [ ] Non-obvious knowledge preserved — depends on the Decision Record
      question above; the root cause has no other home once this log is
      deleted.
- [ ] No durable document depends on this log — true; nothing currently
      cites this log by name.
