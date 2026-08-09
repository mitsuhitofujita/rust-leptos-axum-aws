# Action types list design reference

Status: complete

## Request

Design the action types list page next and add it at
`docs/design/html/action-types-list.html`.

Remove the `Numeric unit` label below every action type name because it is
redundant.

## Interpretation

This is a design-reference change, not a runtime implementation. The page is an
authenticated application screen in the `action-type` domain. It must list the
registered name and numeric unit for every action type, link to creation, and
make each existing type available for editing. It inherits the already-defined
mobile shell and shared geometry.

The request does not define an empty state, routes, loading behavior, errors, or
pagination, so those remain outside this reference rather than being silently
settled here.

## Plan

1. Reuse the exact authenticated header, standard heading, shell, and footer
   geometry from the existing references.
2. Add a prominent creation link and a scannable ordinary-surface list whose
   rows expose name, unit, and edit navigation.
3. Update Page Layouts and Visual Design to register the new reference.
4. Validate HTML structure, shared-block equality, and narrow-width behavior.
5. Ask the requester to confirm the Design Document drafts before retiring this
   log.

## Progress

### 2026-08-09

Added `action-types-list.html`. The populated reference shows six action types.
The primary creation link sits before the collection, and every list row is a
single edit target. The row carries only the domain data currently defined for
an action type — its name and numeric unit — plus decorative navigation cues.
Long names truncate while units remain visible and unwrapped.

Removed the repeated `Numeric unit` secondary label from every row after review.
The unit value remains right-aligned, so the same information is preserved with
less visual noise and each name is vertically centered in its row.

## Verification

`xmllint --html --noout` accepts the new reference without errors, and `git
diff --check` reports no whitespace errors. Direct block comparisons against
`dashboard.html` confirm that the authenticated top row, main gutter, standard
heading, and content-rich footer CSS are byte-for-byte identical.

At the 320-pixel minimum viewport, the shell leaves 268 pixels inside its page
gutters. After row padding, icons, and named gaps, the flexible name column
retains space while long names truncate there and the unit and edit cue keep
their fixed sizes. The full-width creation control also fits inside the same
gutters. At heights of 720 pixels or less, only named section gaps contract and
the populated list uses normal document scrolling.

A browser screenshot remains unavailable in the restricted environment.
Requester confirmation of the Design Document drafts is pending.

## Retirement

- [x] Design Documents updated and confirmed — confirmed by the requester on 2026-08-09
- [x] Decision Records written — none required; this applies established patterns
- [x] Non-obvious knowledge preserved — unspecified states remain explicit in Page Layouts
- [x] No durable document depends on this log
