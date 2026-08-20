# Loosen title letter-spacing

Status: in progress
Started: 2026-08-20
Branch: main

## Request

The page title text feels too tightly tracked; adjust its letter-spacing to a
more natural value, applied consistently across every page. If loosening the
tracking causes a title to lose enough horizontal capacity that it would wrap
onto an extra line where it currently doesn't, shrink that title's font size
rather than letting it wrap. The application's visual design is built on a
golden-ratio scale, so any font-size change made for this reason should stay
in harmony with that scale rather than introduce an arbitrary one-off size.

### Clarifications

Scope confirmed to include both the site-wide page title (the `h1` element,
42px, one shared rule that applies identically on every screen) and the
26px section/dialog titles (the `h2` elements used for subsection headings,
the dashboard link card, and dialog headers such as delete confirmation, the
icon picker, and the type picker).

## Interpretation

"Title" covers two groups of rules in `style/main.css`:

- The site-wide `h1` rule (`font-size: var(--font-display)` = 42px,
  `letter-spacing: -0.065em`), which renders every page's main heading:
  `Dashboard`, `Action types`, `Actions`, the two-line create/edit-form
  headings, both home compositions' headline, and the 404 heading.
- Every `h2` rule sized at `var(--font-title)` (26px): `.dashboard-card h2`
  (-0.045em, the "See every action in one place." link card on signed-in
  home), `.section-heading h2` (-0.035em, "Recent actions" / "Your types" /
  "Your actions"), `.confirm-dialog h2` (-0.045em, the delete-confirmation
  dialogs), and `.icon-dialog h2, .type-dialog h2` (-0.045em, the icon and
  type picker dialog headers).

Out of scope: `.danger-zone h2` (16px, body-weight inline heading, not a
title-scale element) and eyebrow/caption/label text, which is a different,
already-positive-tracking role covered by the design's "Caption" row rather
than "title".

The four 26px `h2` rules are already inconsistent with each other — three use
`-0.045em`, one uses `-0.035em` — which reads like drift rather than an
intentional distinction between roles. Plan to converge on one tracking value
per size unless a visual check surfaces a reason to keep them apart.

"Natural" is a visual judgment, not a formula. Values will be picked by
rendering the affected screens in the dev server and comparing candidates
side by side, rather than derived analytically. The chosen values should
still read as "tight" / "very tight tracking" per the Typography table in
`docs/design/visual-design.md` — the intent recorded there is not changing,
only the degree to which today's values overshoot it.

The golden-ratio type scale has exactly one named step below display (42px):
title (26px), itself ≈ 42 / φ. If loosened tracking pushes a page's existing
single-line `h1` text (e.g. `Action types`, or one line of a deliberately
two-line phrase like `Create a` / `new type.`) onto a second line, the
harmonious fallback is to drop that instance to the existing 26px token
rather than invent a new in-between size. This is not expected to be needed
for the short, fixed English titles already in the app, but will be checked
by rendering every page at the shell's supported widths (320–430px) both
before and after the change.

The dynamic greeting title (`Hello,` / `{display name}.`) is a separate,
pre-existing concern: it already relies on `overflow-wrap: break-word`
specifically because an arbitrary display name can't be bounded (see the
comment on the `h1` rule). Loosened tracking makes overflow marginally more
likely for unusually long names, but resizing dynamic user content per name
is not something the fixed golden-ratio scale can express — treating that as
already handled by the existing wrap fallback, not something this change
needs to alter.

No existing Decision Record governs letter-spacing values, so this is a first
tuning pass rather than a reversal of a recorded decision.
`docs/design/visual-design.md`'s Typography table records only qualitative
tracking ("tight", "very tight tracking"), no exact em values, so it likely
needs no edit — will revisit if the change ends up altering that qualitative
description itself.

## Plan

1. ~~Start the local dev server (`just dev-web`) and render each affected
   screen — signed-out home, signed-in home (including the dashboard link
   card), dashboard, action types list, add/edit action type, actions list,
   add/edit action, the icon-picker and type-picker dialogs, the
   delete-confirmation dialogs, and the 404 page — at the shell's supported
   widths.~~ **Superseded (2026-08-20):** this sandboxed dev container has no
   browser or graphics stack at all (no `chromium-cli`, no Node/Playwright, no
   `fontconfig`, and the `claude-in-chrome` skill reported no extension
   connected in this session) — there is no way to render and look at a page
   from here. The user confirmed the same and will do the visual "does this
   look natural" check themselves against the already-running dev server.
   Wrap-risk checking (the concrete, non-aesthetic half of step 1's goal) is
   replaced by step 1a below.
1a. Download the real Inter ExtraBold (weight 800) TTF and write a small
    throwaway Rust tool (kept in the session scratchpad, not the repo — this
    is a one-off measurement, not a project tool) that sums real glyph
    advance widths plus letter-spacing for every fixed title string in the
    app, and compares each against that title's real available width (shell
    content width, dialog padding, sibling close-button width, all read from
    `style/main.css`) at the narrowest supported viewport (320px). Run it for
    today's values and for candidate looser values.
2. Try a handful of looser `letter-spacing` values for the `h1` rule and for
   the four 26px `h2` rules; compare against today's tracking and settle on
   one value for 42px and one value for 26px that reads natural without
   losing the tight/assertive character the design calls for.
3. Re-check every title from step 1 against the chosen values for line-wrap
   regressions. Where a title that fits one line today would wrap, drop that
   instance to `--font-title` (26px) — the existing golden-ratio step down
   from display — rather than a new size.
4. Apply the chosen values in `style/main.css`.
5. ~~Re-render every affected screen to confirm no unintended wraps or
   overflow.~~ **Superseded (2026-08-20):** same reason as step 1 — no
   renderer available here. Confirm instead that the dev server serves the
   updated CSS (`curl` the built bundle), and hand the running server off to
   the user for the visual/wrap check a real browser would give.
6. Record what was found and verified below; note whether
   `docs/design/visual-design.md` needs an update and whether the 26px
   tracking inconsistency found while reading the CSS is worth a line in a
   Decision Record if a reason for it turns up.

## Progress

### 2026-08-20

Confirmed the full set of rules in scope by reading `style/main.css` and
cross-checking against every `<h1>`/`<h2 id="...">` in `crates/app/src`:

- `h1` (site-wide, 42px): `Dashboard`, `Action types`, `Actions`, both home
  compositions' headline, the four create/edit form headlines, `Not found.`
- `.dashboard-card h2` (26px, 235px max-width): "See every action in one
  place." on signed-in home.
- `.section-heading h2` (26px): `Recent actions`, `Your types`,
  `Your actions`.
- `.confirm-dialog h2` (26px): the delete-confirmation dialog questions.
- `.icon-dialog h2, .type-dialog h2` (26px): `Choose an icon`,
  `Choose a type`.

Tried to follow the Plan's step 1 (render every page in a browser, compare
tracking candidates by eye) and hit a hard environment limit: this dev
container has no browser or graphics stack — no `chromium-cli`, no
Node/Playwright, no `fontconfig`, no system fonts of any kind. The
`claude-in-chrome` skill reported no extension connected in this session, and
the user separately confirmed it can't work from inside this container and
that they'll do the visual check themselves once the dev server is running.
Marked the affected Plan steps superseded above rather than silently
dropping the verification goal they existed for.

For the concrete (non-aesthetic) half of that goal — will loosening the
tracking make any existing title wrap where it doesn't today — substituted a
real measurement in place of the unavailable rendering: fetched Inter
ExtraBold (weight 800, matching the design's stated title weight) as a raw
TTF via `fonts.googleapis.com`'s legacy-user-agent CSS response (which serves
a plain `.ttf` URL for old UAs instead of `.woff2`), and wrote a ~70-line
Rust tool against `ttf-parser` (scratchpad only, not committed — see the
Plan's step 1a) that sums each title string's real glyph advances plus
letter-spacing and compares the total to that title's actual available width
at the narrowest supported viewport (320px), derived from
`style/main.css`'s own shell/gutter/dialog-padding/close-button values.

Ran it for today's values and for a set of candidates, and settled on:

- `h1`: `-0.065em` → `-0.03em` (roughly halves the negative tracking).
- All four 26px `h2` title rules, previously split `-0.045em` (three of
  them) / `-0.035em` (`.section-heading h2`) — read as drift rather than an
  intentional distinction, per the Interpretation above — converged to one
  shared `-0.02em`.

Both keep a visibly tighter-than-normal, bold-display feel (still negative,
still clearly "tight"/"very tight" per the Typography table in
`docs/design/visual-design.md`), just no longer at the original's extreme.

Measured result: every fixed title string that fits on one line today still
fits after loosening — no case flips from fitting to wrapping, so the
golden-ratio font-size fallback described in the Interpretation was not
needed anywhere. Two dialog headings were already tight at the 320px floor
*before* this change and remain similarly tight after it (not a regression,
and out of this task's scope):

- `.confirm-dialog h2`'s longer phrasing (e.g. "Delete this record?") is
  already close to or past its available width at exactly 320px, both before
  and after.
- `.icon-dialog`/`.type-dialog` `h2` ("Choose an icon" / "Choose a type")
  shares its header row with a close button; at 320px the remaining space was
  already tight before this change too.

Both resolve comfortably at the app's normal/designed width (430px) and at
every width above 320px — this is specifically an edge case of the narrowest
*supported* viewport, unrelated to the tracking change.

Applied the two values across the five rule sites in `style/main.css`.
Restarted `just dev-api` (:3000) and `just dev-web` (:8080) — both were
already running from an earlier attempt but had silently died (a backgrounded
subshell in this environment doesn't outlive the tool call that started it,
unlike a task launched with the harness's own background-task tracking).
Confirmed via `curl` that the dev server's built CSS bundle contains the new
values at all five sites.

The dev servers are left running (`http://localhost:8080`, mock auth per
DR-0008 — unauthenticated screens are reachable immediately, authenticated
ones need whatever the user's own local sign-in path is) for the user's own
visual check.

## Verification

- `curl`ed the dev server's built CSS bundle and confirmed all five target
  rules (`h1`, `.dashboard-card h2`, `.section-heading h2`,
  `.confirm-dialog h2`, `.icon-dialog h2, .type-dialog h2`) serve the new
  `-0.03em` / `-0.02em` values, and only those five — no other
  `letter-spacing` declaration in the file changed.
- `trunk`'s own rebuild succeeded (`applying new distribution` / `success` in
  its log) — the CSS change did not break the build.
- Real-glyph wrap-risk check (Inter ExtraBold, `ttf-parser`, see Progress
  above): no fixed title string flips from fitting to wrapping at the
  narrowest supported viewport.
- Not done here, and left to the user: the actual visual "does this read as
  natural tracking, not just as no-longer-wrapping" judgment, and a look at
  every affected screen in a real browser — this sandbox has none. Dev
  servers are up at `http://localhost:8080` for that check.

## Retirement

- [ ] Design Documents updated
- [ ] Decision Records written (DR-____)
- [ ] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
