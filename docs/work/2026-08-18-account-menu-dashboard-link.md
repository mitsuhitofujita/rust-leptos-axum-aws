# Add a Dashboard entry to the account menu

Status: in progress
Started: 2026-08-18
Branch: main

## Request

Add "Dashboard" to the application's navigation menu.

## Interpretation

**What is being asked.** The application has exactly one menu-shaped
navigation surface: the account menu (`AccountControl` in
`crates/app/src/app.rs`), opened from the avatar at the end of the top row on
every authenticated screen (DR-0029). It currently holds `Action`
(`/actions`), `Action Type` (`/action-types`), a separator, and `Log out`.
The request is to add a fourth entry, `Dashboard` (`/dashboard`), so an
authenticated visitor can reach the dashboard from this menu on any screen,
not only via the "Open dashboard" card on the signed-in home
(`page-layouts.md`'s only currently documented path to `/dashboard`).

DR-0029's own Consequences already anticipate this: "any account-related
destination added later joins the same menu rather than needing a new home
invented for it." No alternative placement was considered for that reason —
this is not a decision with real alternatives to weigh, so it does not
warrant a new Decision Record; it is a Design Document update.

**Out of scope.**
- The signed-in home's own "Open dashboard" card — unaffected, stays as an
  additional, separate path.
- Any change to `/dashboard`'s own content or data. That is already real,
  finished work — see the (kept-past-completion) `2026-08-18-dashboard-real-data.md`.
- Any change to the menu's mechanics (native `<dialog>`, focus handling,
  Escape-to-close) — only its list of entries changes.
- The two other open Work Logs (`2026-08-16-automated-test-strategy.md`,
  `2026-08-17-actions-crud.md`) — unrelated, not touched here.

**Assumptions, flagged for confirmation:**
- **Position in the list.** Placing `Dashboard` first, before `Action`,
  since it is the authenticated overview/landing screen rather than a
  peer of the other two entries. Nothing in `page-layouts.md` or DR-0029
  states an ordering rule beyond the order the existing three already
  appear in, so this is a judgment call, not a derived requirement.
- **Icon.** The existing menu entries (`Action`, `Action Type`) use
  hand-written glyphs from `icons.rs` (`Pulse`, `Tag`), not the generated
  `icon_catalog.rs` — `frontend.md` draws that line deliberately ("drawing
  a chevron from the catalog would mean admitting a whole further category
  of icons nobody may choose"), and the catalog has no `layout-dashboard`
  entry regardless. A new hand-written glyph will be added to `icons.rs`
  following the same pattern as `Pulse`/`Tag`/`LogOut`, not sourced from the
  catalog.

## Plan

1. `crates/app/src/icons.rs`: add a hand-written `Dashboard` glyph component,
   matching the viewBox/stroke conventions the neighboring `Pulse`, `Tag`,
   and `LogOut` components already use.
2. `crates/app/src/app.rs`, `AccountControl`: add an `<A href="/dashboard"
   attr:class="menu-link">` entry using the new glyph, as the first item in
   `.menu-list`, before `Action`. Update the component's doc comment, which
   currently says the menu "has to reach three destinations," to say four.
3. `docs/design/page-layouts.md`: add `account menu ── Dashboard
   ────────────▶ dashboard` to the navigation list, alongside the existing
   `account menu ── Action Type ────────▶` / `account menu ── Action
   ─────────────▶` lines.
4. `docs/design/frontend.md`: update the account-menu paragraph ("It holds
   `Action` (`/actions`), `Action Type`, a separator, and `Log out`") to
   include `Dashboard`.
5. Verify: `just fmt`, `just lint`, `just check`, `just test`; manually open
   the account menu in a running dev build and confirm the new entry
   navigates to `/dashboard` and reads consistently with the other two
   entries.

## Progress

### 2026-08-18
Work Log opened. Interpretation and Plan above are ready for confirmation
before implementation starts.

## Verification

Not yet performed — implementation has not started.

## Retirement

- [ ] Design Documents updated
- [ ] Decision Records written (DR-____) — not expected; see Interpretation
- [ ] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
