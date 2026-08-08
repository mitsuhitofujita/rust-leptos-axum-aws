# Document the layout and visual design

Status: in progress

## Request

Understand the application concept and the three HTML design references under
`.local/`, then create layout and design documentation under `docs/` that
follows the documentation model defined by `docs/README.md`.

## Interpretation

The durable layer must preserve the product concept, shared visual system,
screen layouts, and known navigation without depending on the temporary
`.local/` inputs. The three detailed HTML states are authoritative visual
references for signed-out home, signed-in home, and dashboard. Screens named
only by the concept are documented only to that level; unknown routes and
layouts are not invented.

The concept's ten latest dashboard actions and the mock's ten-day summary are
treated as independent limits: the former controls the list and the latter the
chart. No Decision Record is needed because the task supplies the design rather
than asking the agent to choose among durable alternatives.

## Plan

- Extract the durable visual language from the shared HTML styles.
- Describe the screen inventory and layout contracts separately from styling.
- Add both documents to the Design Document index.
- Check language, links, structure, and coverage against the source inputs.
- Ask a human to confirm the drafted Design Documents, as required by the
  documentation model.

## Progress

### 2026-08-08

- Read `docs/README.md`, the existing Design Documents, `.local/concept.md`, and
  all three `.local/html/*.html` references.
- Drafted `docs/design/visual-design.md` with the mobile shell, golden-ratio
  token scales, palette, typography, recurring surfaces, interaction, motion,
  and accessibility rules.
- Drafted `docs/design/page-layouts.md` with the common anatomy, both home
  states, dashboard, remaining screen requirements, data interfaces, and known
  navigation.
- Updated `docs/design/index.md` to make both documents part of the durable
  reading path.

## Verification

- `git diff --check` reported no whitespace errors.
- Confirmed that every relative Markdown link in the new documents resolves.
- Confirmed that all new document prose is English.
- Checked the durable documents against every concept section and the shared
  tokens, semantic structures, responsive rules, interaction states, and
  reduced-motion rules in all three HTML references.
- Pending human confirmation of the Design Document drafts.

## Retirement

- [x] Design Documents updated
- [x] Decision Records written (none required)
- [x] Non-obvious knowledge preserved
- [x] No durable document depends on this log
- [ ] Design Document drafts confirmed by a human
