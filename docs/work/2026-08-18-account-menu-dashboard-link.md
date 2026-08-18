# Add a Dashboard entry to the account menu

Status: complete
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
   - **Superseded, same day.** No browser tooling is available in this
     devcontainer (no Claude-in-Chrome connection, no `chromium-cli`, no
     `node`/`npm`, no `chromium`/`google-chrome` binary), so a real click
     could not be performed. Replaced with: build the bundle with dummy
     (non-network) `COGNITO_CLIENT_ID`/`COGNITO_HOSTED_UI_DOMAIN` values so
     `AccountControl` compiles into a non-`Disabled` build, then inspect the
     served `dist/*.wasm` artifact directly for the new menu markup and its
     order. See Verification.

## Progress

### 2026-08-18
Work Log opened. Interpretation and Plan above are ready for confirmation
before implementation starts.

### 2026-08-18 (implementation)
Implemented steps 1–4 as planned, no deviation in shape:

- `crates/app/src/icons.rs`: added `Dashboard`, a hand-written glyph (four
  rounded rectangles — Lucide's `layout-dashboard` shape, drawn by hand the
  same way `Pulse`/`Tag`/`LogOut` already are, not sourced from
  `icon_catalog.rs`), placed immediately before `Pulse` so the file's
  ordering matches the menu's.
- `crates/app/src/app.rs`: imported `Dashboard`; added an `<A
  href="/dashboard">` entry as the first child of `.menu-list`, before
  `Action`; updated `AccountControl`'s doc comment from "three destinations"
  to "four destinations — the dashboard, the actions list, the action-type
  area, and signing out."
- `docs/design/page-layouts.md`: added `account menu ── Dashboard
  ───────────▶ dashboard` to the navigation list, above the existing
  `Action Type` and `Action` lines.
- `docs/design/frontend.md`: updated the account-menu paragraph to read "It
  holds `Dashboard` (`/dashboard`), `Action` (`/actions`), `Action Type`, a
  separator, and `Log out`."
- Ran `just fmt`; it reflowed the doc-comment edit's line wrap slightly, no
  other changes.
- Plan step 5's manual-browser portion could not run as originally planned
  — see the superseded note on that step. Verified instead by starting
  `just dev-api` and a `trunk serve --proxy-backend http://127.0.0.1:3000/api`
  with `COGNITO_CLIENT_ID=dummy-client-id` and
  `COGNITO_HOSTED_UI_DOMAIN=dummy.auth.example.com` (both compile-time-only
  values `auth.rs::is_configured()` checks for non-emptiness, never
  dereferenced over the network by anything this check exercises), then
  running `strings` on the produced `dist/app-*_bg.wasm`. It contains the
  literal sequence `/dashboardmenu-link/actions/action-typesmenu-list`,
  confirming both that the new entry compiled in and that its order in the
  menu is `Dashboard`, `Action`, `Action Type` as intended. Both dev
  processes were stopped afterward.

No decision surfaced with durable consequences or real alternatives beyond
what Interpretation already flagged (list position, hand-written icon) —
both were judgment calls, not contested trade-offs, so nothing here rises to
a Decision Record.

## Verification

- `just fmt`: applied cleanly (see Progress).
- `just check`: `cargo check --workspace` and `cargo check -p app --target
  wasm32-unknown-unknown` both clean.
- `just lint`: `cargo clippy --workspace --all-targets -- -D warnings` and
  the `app`/wasm32 pass both clean.
- `just test`: 64 passed, 0 failed, 10 ignored — unchanged from before this
  work, as expected for a frontend-only change with no new test coverage
  added.
- Manual verification of the actual rendered menu (click the avatar, see
  the entry, follow it to `/dashboard`) was **not performed** — this
  container has no browser automation available. What was checked instead:
  the compiled `wasm` artifact contains the new menu-list markup in the
  intended order (see Progress). This confirms the change built and landed
  in the served bundle; it does not confirm the glyph renders correctly or
  that the click-through behaves as expected in an actual browser.

## Retirement

- [x] Design Documents updated — `page-layouts.md`, `frontend.md`
- [x] Decision Records written (DR-____) — not expected; see Interpretation
- [x] Non-obvious knowledge preserved — nothing arose beyond the two
      judgment calls already recorded in Interpretation (list position,
      hand-written icon); neither is a rejected alternative or a pitfall
      worth a Decision Record
- [x] No durable document depends on this log
