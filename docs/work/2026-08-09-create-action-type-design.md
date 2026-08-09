# Create action type design reference

Status: complete

## Request

Understand the project's documentation model, consult the durable design
documents and existing HTML references, and design the create action type page
described in the local application concept. Add the result at
`docs/design/html/action-categories-create.html`. Keep the existing header,
footer, and content-title geometry pixel-aligned with the other pages. Respond
to the requester in Japanese.

The English domain name is `action-type`; use that term consistently, including
in the HTML reference file name.

## Interpretation

This unit adds a design reference, not a runtime route or component. The page is
an authenticated application screen and therefore reuses the dashboard's exact
top row, standard heading geometry, user image, shell, and footer. Its two
required domain inputs are the English action name and numeric unit. Form
submission, routes, validation messages, and persistence remain outside this
design-only request.

The new reference resolves the detailed composition that Page Layouts had
previously left unspecified, so the relevant durable design documents receive
draft updates and require human confirmation before this log can retire.

## Plan

1. Read the documentation model, design index, page layouts, visual design,
   application concept, and every existing HTML design reference.
2. Build an accessible, English-only, mobile form using the existing design
   tokens and pixel-identical shared shell geometry.
3. Update Page Layouts and Visual Design to register the new current design.
4. Validate the HTML and compare rendered geometry at supported viewport sizes.
5. Ask the requester to confirm the Design Document updates, then complete and
   retire this log.

## Progress

### 2026-08-09

Read the documentation model and the durable design entry path. The shared
authenticated shell was taken from `dashboard.html`: a 430-pixel maximum shell,
26-pixel gutter, 42-pixel top row, 26-pixel heading gap, standard eyebrow and
42-pixel display title, and the same authenticated avatar control. The new form
uses an ordinary translucent content surface, two clearly labelled text inputs,
a solid accent primary action, and a lower-emphasis cancel link.

Added the reference initially as `action-categories-create.html`, following the
original requested path while using the product's established “action type”
term in user-facing copy. After the English domain name was confirmed as
`action-type`, renamed the reference to `action-types-create.html` and updated
its durable-document link.

## Verification

`xmllint --html --noout` accepts the new reference without errors, and `git
diff --check` reports no whitespace errors. Direct block comparisons against
`dashboard.html` confirm that the authenticated top row and standard heading
CSS are byte-for-byte identical. The footer block is byte-for-byte identical
to the short-screen footer in `home-withauth.html`.

The layout was also checked arithmetically at the supported 320- and 430-pixel
widths: both inputs and the primary button remain within the 26-pixel page
gutters. At heights of 720 pixels or less, the existing media-query breakpoint
contracts only named spacing tokens and permits normal document scrolling.

A headless Firefox screenshot could not be captured in the restricted
environment. Requester confirmation of the Design Document drafts remains
pending.

## Retirement

- [x] Design Documents updated and confirmed — confirmed by the requester on 2026-08-09
- [x] Decision Records written — none required; this applies the existing visual language
- [x] Non-obvious knowledge preserved — none beyond the current design
- [x] No durable document depends on this log
