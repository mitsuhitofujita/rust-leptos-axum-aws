# Account menu navigation from the dashboard avatar

Status: complete
Started: 2026-08-16
Branch: main

## Request

Earlier in this session, a design file was added at
`docs/design/html/dashboard-navigation.html`. It shows an authenticated
screen's top-row account image opening a menu that slides in from the right,
containing `Action`, `Action Type`, a separator, and `Log out`. `Action` points
at `/actions`, which has no screen yet and is expected to resolve to the
existing not-found fallback; `Action Type` points at `/action-types`. Margins,
font sizes, and spacing were matched pixel-for-pixel against the project's
other HTML design references, following the same golden-ratio-derived token
scale those references already use.

The user then asked to implement the navigation — realizing that design in the
actual Leptos application.

## Interpretation

**What is being asked.** Build the account menu shown in
`dashboard-navigation.html` inside `crates/app`:

- Replace `AccountControl` (`crates/app/src/app.rs`) — currently a direct link
  from the avatar to `/action-types` — with a control that opens a menu.
- The menu holds four entries in order: `Action` (`/actions`), `Action Type`
  (`/action-types`), a separator, `Log out`.
- `AccountControl` is shared by every authenticated screen (dashboard, the
  action-types index, creation, and edit screens), so this one change reaches
  all of them.
- Add the corresponding styling to `style/main.css`, reusing the existing
  design tokens and matching the pixel values already chosen in the HTML
  reference (266px drawer width, 56px row height, 42px icon tiles, and so on).

**Out of scope.**

- The `/actions` screen itself (the actions list). No route is added for it;
  the link is expected to keep landing on the router's existing `NotFound`
  fallback, exactly as the design reference intends.
- Any change to what the actions list or an "add action" screen will look
  like — `page-layouts.md` already tracks those as screens without a defined
  layout.

**Assumptions.**

- The dialog stays native. `frontend.md` documents that both the icon picker
  and the delete-confirmation dialog use a real `<dialog>` with `showModal` /
  `close`, specifically so focus containment and Escape-to-close come from the
  browser instead of being written by hand. The menu will follow the same
  pattern rather than a hand-rolled `position: fixed` overlay — CSS turns the
  dialog into a right-edge, full-height drawer and dims `::backdrop`, instead
  of centering it like the delete-confirmation dialog does.
- `Log out` calls the existing `auth::sign_out()` (already used by
  `home.rs`'s `SignedInHome`). No new sign-out logic is needed; the existing
  `RequireAuth` guard already sends a visitor home once the auth state flips
  to `SignedOut`.
- **This overturns an existing, explicitly documented decision.** The current
  `AccountControl` doc comment reads: *"Page Layouts requires it to reach the
  action-type area, and it is the only route to that area from the dashboard,
  so it links there directly rather than opening a menu with one entry."*
  That reasoning holds only while the menu would have one entry; once it
  needs to reach `Action`, `Action Type`, and `Log out`, a menu is no longer
  "opening a menu with one entry" but the more natural way to expose three
  destinations from one control. The doc comment will be rewritten to say why
  a menu now, not just deleted. Given a documented decision is being
  reversed with real trade-offs (direct link and its instant navigation vs. a
  menu with an extra tap), **this looks like it warrants a Decision Record**
  — proposed once the implementation is settled, rather than assumed here.
- `page-layouts.md`'s navigation diagram currently shows only
  `authenticated avatar ──▶ action-type access`, with no `Log out` or
  `Action` entries from an authenticated screen. Realizing this design will
  leave that diagram and the Dashboard section out of date. A draft update
  will be prepared, but per `docs/README.md`'s ownership rule a human
  confirms a Design Document overwrite before the work is considered
  complete — it will be presented, not applied silently.
- The menu's `Action`, `Action Type`, and `Log out` icons are decorative UI
  chrome, not action-type icons — `frontend.md` draws that line already
  ("Everything else in `icons.rs` ... is hand-written, because drawing a
  chevron from the catalog would mean admitting a whole further category of
  icons nobody may choose"). New hand-written SVGs will be added to
  `icons.rs` alongside the existing ones, not sourced from the generated
  `icon_catalog.rs`.

## Plan

1. `crates/app/src/app.rs` — turn `AccountControl` into a trigger button plus
   a `<dialog>`-based drawer (`NodeRef<Dialog>`, `show_modal()` / `close()`,
   mirroring `EditForm`'s confirm dialog in `action_types.rs`). Rewrite its
   doc comment to explain the menu, not the direct link. The drawer holds:
   `Action` (`/actions`), `Action Type` (`/action-types`), a separator, and a
   `Log out` button calling `auth::sign_out()`.
2. `crates/app/src/icons.rs` — add the three hand-written glyphs the drawer
   needs (an activity-style mark for `Action`, a tag mark for `Action Type`,
   a log-out mark), matching the SVGs already drafted in
   `dashboard-navigation.html`.
3. `style/main.css` — add the drawer's styling: the golden-ratio drawer width
   (266px), the dimmed `::backdrop`, the right-edge full-height `<dialog>`
   positioning, and the nav-link/separator/account-summary rules, reusing the
   existing spacing and color tokens and matching the reference file's pixel
   values.
4. Draft the `page-layouts.md` update (Dashboard section prose and the
   navigation diagram) reflecting the menu, and present it for confirmation
   rather than applying it unannounced.
5. Run the project's existing checks — `just fmt-check`, `just lint`,
   `just check`, `just test` — and start `just dev-web` (or
   `just dev-web-auth` if sign-in needs to be exercised) to verify in the
   browser: the avatar opens the drawer, `Action Type` navigates correctly,
   `Action` lands on the not-found fallback, `Log out` returns to `/`, and
   Escape / the native backdrop close the drawer.
6. Once the implementation is settled, propose the Decision Record covering
   the direct-link → menu reversal, and confirm whether the user wants one
   written.

## Progress

### 2026-08-16

Read `docs/README.md`, the Design Document index, `frontend.md`, and
`page-layouts.md`. Read the current implementation: `app.rs` (`AccountControl`,
routing, `RequireAuth`), `dashboard.rs`, `home.rs`, `action_types.rs`,
`auth.rs`, and the relevant slice of `style/main.css`. Confirmed
`AccountControl` is presently a direct `<A href="/action-types">` with a doc
comment explicitly reasoning against a menu, and that `/actions` has no route
today — the router's `<Routes fallback=NotFound>` is what a link to it will
hit. Wrote Interpretation and Plan above; stopped for confirmation before
implementing, per the Work Log skill's instructions.

The user confirmed the direct-link reasoning was never a decision they made,
and asked to overturn it and implement. Proceeded with the plan:

- `crates/app/src/icons.rs`: added `Pulse` (Action), `Tag` (Action Type), and
  `LogOut`, hand-written alongside the existing chrome icons rather than
  pulled from the generated action-type catalog.
- `crates/app/src/app.rs`: rewrote `AccountControl`. The trigger is now a
  `<button>` (`aria-haspopup="dialog"`, `aria-controls`, `aria-expanded`)
  instead of an `<A>`; it opens a `<dialog class="menu-dialog">` with
  `show_modal()`, mirroring `EditForm`'s confirm dialog and `IconField`'s
  picker — `NodeRef<Dialog>`, an `on:close` handler returning focus to the
  trigger, nothing hand-rolled for focus containment or Escape. The dialog
  holds `Action` (`/actions`, via `<A>`), `Action Type` (`/action-types`, via
  `<A>`), an `<hr>` separator, and a `Log out` `<button>` that closes the
  dialog and calls the existing `auth::sign_out()`. Rewrote the doc comment
  to explain the menu instead of the old direct-link reasoning.
- `style/main.css`: added `--menu-width: 266px` to `:root` (golden ratio:
  `--app-width` / φ), matching the ratio `dashboard-navigation.html` used for
  its drawer. Added a `.menu-dialog` block styled like `.confirm-dialog` /
  `.icon-dialog` — native dialog, own `::backdrop`, `showModal`-driven — but
  pinned to the right edge and full height rather than centred, with a
  `menu-in` slide-in animation (already covered by the existing global
  `prefers-reduced-motion` rule, so no extra opt-out needed). `.menu-close`
  reuses `.icon-dialog-close`'s look. `.menu-link`, `.menu-icon`,
  `.menu-separator` reuse the existing spacing/color tokens and match the
  pixel values already chosen in the HTML reference (56px row height, 42px
  icon tiles, 19px glyphs).
- Verified: `cargo check --workspace`, `cargo check -p app --target
  wasm32-unknown-unknown`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo clippy -p app --target wasm32-unknown-unknown -- -D
  warnings`, `cargo fmt --all --check`, and `trunk build` all pass.
- Not verified: actual in-browser behaviour (menu open/close, focus return,
  `/actions` landing on the not-found fallback, `Log out` returning to `/`).
  `claude-in-chrome` is not connected in this container session, so this was
  checked by compiling and reading the rendered logic only, not by driving a
  browser.
- Drafted, not yet applied: a `page-layouts.md` update reflecting the menu in
  the navigation diagram and the shared-anatomy prose — pending user
  confirmation per `docs/README.md`'s ownership rule.
- This reverses `AccountControl`'s previous direct-link design, which was
  reasoned about explicitly in code even though the user says it was never a
  decision they made. Proposed to the user: write a Decision Record for the
  menu, covering the trade-off (one direct tap vs. an extra tap through a
  menu) and why three destinations tipped it.
- The user confirmed the `page-layouts.md` draft. Applied it: the shared page
  anatomy prose now says the avatar "opens an account menu reaching the
  action-type area, the actions list, and signing out"; the navigation
  diagram replaces `authenticated avatar ──▶ action-type access` with the
  avatar opening the menu and three arrows out of it (`Action Type`,
  `Action`, `Log out`); the `Actions` row in "Remaining application screens"
  now notes it is reached from the account menu and lands on the not-found
  fallback until built. Bumped the document's `Updated` date and added the
  index row.
- The user confirmed writing a Decision Record. Wrote
  `docs/decisions/DR-0029-the-authenticated-avatar-opens-an-account-menu.md`,
  covering the context (the direct link's own justification stopped holding
  once three destinations were needed), the decision, the two alternatives
  considered (leaving `Action`/`Log out` elsewhere; a two-entry menu without
  `Action`), and the consequences (one shared entry point vs. an extra tap to
  reach action types, and `Log out` now living in two different places
  depending on the screen). Added it to the Design Document index's Decision
  Records table.

### 2026-08-16 (continued)

The user tried the menu in a real browser: opening it works, but the close
button in the top-right does nothing.

Root cause: `.menu-dialog`'s unconditional `display: flex` in
`style/main.css`. A native `<dialog>` relies on its UA stylesheet
(`dialog:not([open]) { display: none; }`) to disappear once `close()` clears
its `open` attribute — but author styles always outrank UA styles regardless
of specificity, so the unconditional `display: flex` kept overriding that
`display: none` even after closing. The dialog *was* closing — `open` was
being removed and `dismissed` was firing — it just stayed visually on
screen, still `position: fixed` over the right edge, because nothing ever
told it to stop being `display: flex`. `.confirm-dialog` and `.icon-dialog`
never had this problem because neither sets `display` at all, leaving the UA
toggle alone.

Fix: moved `display: flex` (and `flex-direction: column`) from the bare
`.menu-dialog` selector into `.menu-dialog[open]`, alongside the existing
`menu-in` animation. Closed state now falls through to the UA default.
Re-ran `trunk build`; still succeeds.

The user confirmed in the browser that the close button now hides the menu.

The user then pointed out that this Work Log had never been committed to
git, so deleting it (as done immediately after the retirement checklist was
first satisfied) actually destroyed its content instead of leaving it
recoverable through version control, contradicting `docs/README.md`'s "Nothing
is lost: version control retains them." Restored this file from conversation
context. Lesson for future retirement: confirm a Work Log is committed —
not just that the checklist is satisfied — before deleting it, since the
whole safety argument for deleting one depends on it already being in
history.

## Verification

`cargo check --workspace`, `cargo check -p app --target
wasm32-unknown-unknown`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo clippy -p app --target wasm32-unknown-unknown -- -D
warnings`, `cargo fmt --all --check`, and `trunk build` all pass.

The user exercised the menu in a real browser (`claude-in-chrome` was never
connected in this container session, so this checking was entirely on the
user's side). First pass found the close button did nothing — see the
`display: flex` fix above — and after that fix, the user confirmed the close
button now hides the menu.

## Retirement

- [x] Design Documents updated — `docs/design/page-layouts.md` (navigation
      diagram and shared anatomy prose) and `docs/design/frontend.md` (the
      account menu's structure, and the `<dialog>` `display` constraint the
      close-button bug taught)
- [x] Decision Records written — DR-0029
- [x] Non-obvious knowledge preserved — the rejected alternatives and
      trade-offs live in DR-0029; the `display`-on-`[open]` pitfall lives in
      `frontend.md`'s Constraints
- [x] No durable document depends on this log

Implemented, styled, documented, and confirmed working in a real browser by
the user, including the close-button regression found and fixed along the
way, and including the Work Log deletion mistake this section itself records.
Everything of durable value has a home elsewhere; this file should not be
deleted again until it, and everything else changed in this unit of work, is
committed.
