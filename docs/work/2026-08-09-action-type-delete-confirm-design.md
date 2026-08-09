# Delete action type confirmation design reference

Status: complete

## Request

Add `docs/design/html/action-types-delete-confirm.html` as a UI state in which
the edit action type page displays a custom confirmation dialog.

## Interpretation

This unit defines the confirmation presentation required by the edit page's
delete trigger. It does not implement deletion. The custom dialog must identify
the selected type, explain the immediate known consequence, expose explicit
safe and destructive choices, and make the underlying edit screen unavailable
while open.

The effect on historical records, server behavior, success and error feedback,
post-delete navigation, dismissal by Escape or the backdrop, and focus return
remain unspecified. The safe action receives initial focus in the static
reference.

## Plan

1. Compose the dialog state directly over the existing edit HTML reference so
   that its background cannot drift from the base page.
2. Build a semantic, custom-styled modal with a dimmed and blurred shell,
   selected-type summary, consequence copy, and two explicit actions.
3. Update Page Layouts and Visual Design to register the state and its current
   behavioral boundary.
4. Validate HTML structure, narrow-viewport fit, and reduced-motion behavior.
5. Ask the requester to confirm the Design Document drafts before retiring this
   log.

## Progress

### 2026-08-09

Added `action-types-delete-confirm.html`. It embeds the inert edit reference as
its exact background and overlays a semantic open dialog inside a 430-pixel
maximum layer. The layer darkens and lightly blurs only the application shell,
not the outer canvas. The dialog names `Running`, repeats its `km` unit, states
that the type becomes unavailable for new records, and warns that deletion
cannot be undone.

The safe `Keep action type` button comes first and receives initial focus. The
confirmed delete action uses solid ink rather than the product accent, because
red and pink already communicate product identity rather than error. Labels and
a trash glyph carry the destructive meaning without depending on color.

## Verification

`xmllint --html --noout` accepts the new reference without errors, `git diff
--check` reports no whitespace errors, and the referenced
`action-types-edit.html` exists at the relative path used by the preview frame.
The modal carries an accessible name and description, declares modal state, and
the underlying frame is removed from focus and the accessibility tree.

At the 320-pixel minimum viewport, the layer leaves a 268-pixel dialog width and
the standard 26-pixel card padding leaves 216 pixels for content. The type
summary and both buttons fit that width; copy wraps normally. At heights of 720
pixels or less, the layer and card contract to 16-pixel vertical padding and the
layer permits vertical scrolling. Reduced-motion rules cover both veil and
dialog entrance animations.

A browser screenshot remains unavailable in the restricted environment.
Requester confirmation of the Design Document drafts is pending.

## Retirement

- [x] Design Documents updated and confirmed — confirmed by the requester on 2026-08-09
- [x] Decision Records written — none required; unresolved behavior remains explicit
- [x] Non-obvious knowledge preserved — composition and behavioral boundaries are durable
- [x] No durable document depends on this log
