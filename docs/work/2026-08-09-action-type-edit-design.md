# Edit action type design reference

Status: complete

## Request

Design the action type edit page at
`docs/design/html/action-types-edit.html`, including a delete button.

## Interpretation

Although the request repeats “action types list,” the requested `edit` file name
and required delete control identify this as the edit screen already present in
the product inventory. This unit creates a design reference, not runtime edit or
delete behavior.

The form edits the two defined action-type fields, name and numeric unit. The
save action is primary, cancellation returns without saving, and deletion is
visually separated from routine editing. The request requires a delete button
but does not settle confirmation, referential-integrity behavior for existing
records, routes, validation errors, or API behavior; those remain unspecified.

## Plan

1. Reuse the exact authenticated shell, top row, standard heading, form fields,
   and content-rich footer from the existing references.
2. Show representative current values, a primary save action, cancellation,
   and a clearly labelled but non-primary delete area.
3. Update Page Layouts and Visual Design to register the new reference without
   inventing deletion behavior.
4. Validate HTML, shared-block equality, focus treatment, and narrow-width flow.
5. Ask the requester to confirm the Design Document drafts before retiring this
   log.

## Progress

### 2026-08-09

Added `action-types-edit.html`. Its form is prefilled with the same `Running` and
`km` example used across the references. Save remains the solid accent primary
action. Delete is a separate outlined button after a divider and explanatory
copy, preventing it from competing visually with the routine save path or being
mistaken for part of form submission.

## Verification

`xmllint --html --noout` accepts the new reference without errors, and `git
diff --check` reports no whitespace errors. Direct block comparisons against
`dashboard.html` confirm that the authenticated top row, main gutter, standard
heading, and content-rich footer CSS are byte-for-byte identical. Both form
controls retain the create page's field dimensions and focus treatment.

At the 320-pixel minimum viewport, the shell leaves 268 pixels inside its page
gutters and 216 pixels inside the form card at standard spacing; inputs and all
three full-width or centered actions fit without horizontal overflow. The edit
and deletion sections use normal document scrolling, and heights of 720 pixels
or less contract only named gaps and card padding.

A browser screenshot remains unavailable in the restricted environment.
Requester confirmation of the Design Document drafts is pending.

## Retirement

- [x] Design Documents updated and confirmed — confirmed by the requester on 2026-08-09
- [x] Decision Records written — none required; deletion behavior remains unspecified
- [x] Non-obvious knowledge preserved — behavioral boundaries are in Page Layouts
- [x] No durable document depends on this log
