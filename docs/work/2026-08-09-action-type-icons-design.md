# Configurable action type icons

Status: complete

## Request

Allow an icon to be configured on an action type. Reflect each configured icon
at the left of its row in `docs/design/html/action-types-list.html`, and add icon
input to `docs/design/html/action-types-create.html` and
`docs/design/html/action-types-edit.html`.

Replace the inline button grid because it will become difficult to use as the
number of icons grows. Follow icon-selection best practices in the create and
edit references.

Remove the visible `Selected`, `Change`, and current icon name from the compact
selector. In the expanded picker, use each icon's actual name. Evaluate whether
Lucide can supply the icon catalog.

## Interpretation

This unit extends the action-type design model from name and numeric unit to
name, numeric unit, and icon. It updates design references only; persistence,
shared types, APIs, routes, and runtime components remain outside the request.

The input uses a required built-in icon set rather than uploads or raw SVG. That
choice keeps the existing line-icon language consistent and introduces no media
lifecycle. Because this affects the future persisted representation and carries
real alternatives, it is recorded in DR-0012.

## Plan

1. Add the same accessible eight-option icon radio group to create and edit,
   with the representative running icon selected. **Superseded by step 6.**
2. Replace the generic tag glyph in each list row with that type's configured
   icon, using the established dashboard paths.
3. Update Page Layouts, Visual Design, the design index, and a Decision Record to
   make the extended domain model durable.
4. Validate all affected HTML, radio-group semantics, shared geometry, and the
   four-column picker at the 320-pixel minimum width.
5. Ask the requester to confirm the Design Document drafts before retiring this
   log.
6. Supersede the reviewed inline grid with a compact selected-value field and a
   searchable modal picker, preserving the built-in identifiers while revising
   DR-0012 through DR-0013.
7. Evaluate Lucide against the existing visual and technical constraints, then
   simplify the collapsed selector and align picker labels, stored identifiers,
   list glyphs, durable documents, and license notices with the result.
8. Review the durable layer against the documentation model, repair the
   supersession pointers, and record what the intended `lucide-leptos` adoption
   costs.

## Progress

### 2026-08-09

Added matching eight-choice icon pickers to the create and edit references. The
choices are Running, Water, Reading, Meditation, Cycling, Strength, Study, and
Walking. Each visible symbol is paired with visually hidden text inside its
radio label. Checked options use the accent surface, and keyboard focus uses the
standard three-pixel accent outline.

Replaced the action types list's generic tag glyphs with configured activity
glyphs matching the corresponding action names. The icons remain hidden from
assistive technology in rows because the adjacent visible names already identify
the targets.

Wrote DR-0012 for the built-in-set decision and drafted the corresponding design
document updates.

After review, the inline grid was found not to scale with the icon library.
Replaced it in both forms with one compact field showing the current glyph and
name. The field opens a native modal dialog whose normal search input filters a
scrollable, visibly labelled radio list. Selection is staged behind `Use
selected icon`; close and Escape preserve the former value, and focus returns to
the invoking field. DR-0013 supersedes only the presentation portion of DR-0012
while preserving its built-in identifier model.

Evaluated Lucide's official catalog, SVG distribution model, and license. Its
24-pixel line geometry matches the existing small glyph containers, and its
static SVGs can be rendered without a browser-side package or third-party
request. Pinned the design decision and notice to Lucide 1.30.0. Adopted
canonical kebab-case Lucide names as identifiers and official human-readable
Lucide names as picker labels. The initial examples are `Person Standing`,
`Droplets`, `Book Open`, `Timer`, `Bike`, `Dumbbell`, `Graduation Cap`, and
`Footprints`.

Reduced the collapsed form control to its icon alone, removing the visible
`Selected`, `Change`, and current-name copy. The visible `Icon` field label
continues to identify the control, and the current official name remains
visually hidden as part of its accessible name. Replaced the list examples with
the same pinned Lucide SVG geometry and added the upstream license notice.
DR-0014 records the catalog, naming, rendering, and licensing decision.

## Verification — inline grid (superseded)

`xmllint --html --noout` accepts all three affected references without errors,
and `git diff --check` reports no whitespace errors. XPath checks find exactly
eight `icon` radio controls in both create and edit, one required checked choice
in each, and six configured icon containers for the six list rows. The former
generic tag path is absent from the list.

Direct block comparisons confirm that the authenticated header and standard
page heading remain byte-for-byte identical to `dashboard.html`. The list and
edit content-rich footers match the dashboard footer, while the create footer
continues to match the short-screen signed-in home footer.

At the 320-pixel minimum viewport, standard form-card padding leaves 216 pixels.
After three 10-pixel grid gaps, each of the four picker columns is 46.5 pixels
wide and 56 pixels high, so the controls fit without horizontal overflow. At
the short-height breakpoint, reduced card padding increases their available
width. Keyboard focus, checked state, and reduced-motion behavior are defined.

A browser screenshot remains unavailable in the restricted environment.
Requester confirmation of the Design Document drafts is pending.

## Verification — searchable modal picker

Initial status: pending final HTML validation, native-dialog interaction checks,
modal accessibility checks, and narrow-width verification. The earlier grid
geometry above is retained as the verification record for the superseded design
and no longer describes the current references.

`xmllint --html --noout` accepts both current form references, `git diff --check`
reports no whitespace errors, and `node --check` accepts both embedded picker
scripts. XPath checks find one submitted icon value, one modal, one normal search
input, eight named radio choices, and one checked choice in each document.

The trigger has a visible label, current glyph and name, `aria-haspopup`,
`aria-controls`, expanded state, and helper description. The modal has a visible
title, declares modal state, includes a visible close button, focuses search on
open, relies on the native dialog for Escape and focus containment, and restores
focus to the trigger on close. Filtering announces its count and shows an empty
state. A filtered-out checked value disables application until a visible result
is selected.

At the 320-pixel minimum viewport, the compact form field has 216 pixels inside
the card. Its 42-pixel preview and fixed `Change` affordance leave the current
name a flexible truncation-safe column. The dialog is 268 pixels wide with 216
pixels inside standard padding; its result rows have enough space for the
42-pixel preview, every current English name, and the 26-pixel checked marker.
The result area scrolls independently and contracts with dynamic viewport
height.

Direct comparisons reconfirm that both pages retain the exact authenticated
header and standard heading blocks. Edit retains the content-rich footer and
create retains the short-screen footer. A rendered browser interaction remains
unavailable in the restricted environment. Requester confirmation of the Design
Document drafts is pending.

## Verification — Lucide and icon-only revision

The preceding trigger-width description records the superseded labelled
selector; the current 68-pixel icon-only control no longer contains a flexible
name column or `Change` affordance.

`xmllint --html --noout` accepts create, edit, list, and dashboard without
errors, and `node --check` accepts both embedded picker scripts. Each form has
one submitted `person-standing` value, one modal, one search input, eight radio
choices, one checked choice, and one visually hidden current-name node. Searches
confirm that the form references no longer contain visible `Selected`, `Change`,
or the former activity-oriented option labels and identifiers.

The eight picker labels and values match in both references: `Person Standing`
(`person-standing`), `Droplets` (`droplets`), `Book Open` (`book-open`), `Timer`
(`timer`), `Bike` (`bike`), `Dumbbell` (`dumbbell`), `Graduation Cap`
(`graduation-cap`), and `Footprints` (`footprints`). The list contains six and
the dashboard contains ten configured activity glyphs using the Lucide
24-pixel view box and two-pixel stroke treatment; no former 1.8-pixel activity
glyph remains in either reference.

Direct block comparisons show that create and edit retain the dashboard's exact
authenticated header and page-heading CSS. Edit retains the dashboard footer,
and create retains the short-screen signed-in-home footer. Markup and document
checks find no trailing whitespace, while `git diff --check` accepts tracked
changes. A rendered browser interaction remains unavailable in the restricted
environment. Requester confirmation of the Design Document drafts is pending.

### 2026-08-09 — durable-layer review

Reviewed the drafted durable layer against `docs/README.md` and corrected four
things it got wrong.

DR-0012's Status pointed at DR-0013, which does not supersede it — DR-0013
explicitly continues DR-0012's stored identifier and reuses its rejections. The
record that actually replaced DR-0012 is DR-0014, which swapped the
application-owned catalog for Lucide, so the Status now points there and
DR-0014 says which part of DR-0012 it replaces and which parts survive. The
design index marks the row superseded, a notation this project had not needed
before. DR-0014 also now states that it revises DR-0013's collapsed selector to
icon-only, which was otherwise a silent contradiction between two accepted
records.

Checked the two version claims rather than trusting them. Lucide 1.30.0 is real
— the current `lucide` release on npm. `lucide-leptos` is at 3.26.0 and depends
on `leptos ^0.8.0`, which is the line DR-0002 keeps this workspace on. The crate
version does not encode the Lucide version it vendors, so DR-0014 now names both
pins and says what each one governs.

Two consequences of the intended `lucide-leptos` adoption were missing and are
now in DR-0014. Its icons are generated components rather than a runtime lookup,
so the stored kebab-case name needs a generated mapping — the same table the
picker searches. And its features gate icons by category, not individually, so a
catalog broad enough to justify a search box is paid for in bundle size.

Page Layouts gained DR citations on its icon constraints, lost a sentence
explaining an absent label — rationale for a rejected alternative, which belongs
in a Decision Record and not in a document that describes the present — and
gained the icon-picker transitions its navigation list had omitted while
including the deletion-confirmation ones. The delete-confirmation diagram's top
border was one character wide.

Visual Design now carries `68px` in the spacing table as the golden step after
`42px` (`26 + 42`), which is what the action-type rows and the icon selector
already use. `56px` is recorded beside it as a touch-target minimum for
full-width controls, deliberately not as a spacing step: it predates this work
in `.google-button` and does not sit on the progression.

The license notice was a paraphrase. It is now verbatim from upstream — the
earlier text had dropped the list of Feather-derived icons that the MIT block
applies to. Its path is named in DR-0014 and in Visual Design's Interfaces, so
the durable layer points at it rather than leaving it unreferenced.

Tailwind was raised as a possible adoption and then withdrawn, so the
`no framework, no npm` constraints in Visual Design and Frontend stand unchanged
and no reversing record was written.

## Retirement

- [x] Design Documents updated and confirmed — confirmed by the requester on 2026-08-09
- [x] Decision Records written — DR-0012, superseded by DR-0014; DR-0013; DR-0014
- [x] Non-obvious knowledge preserved — picker alternatives are in DR-0013; catalog, identifier, rendering, license, and `lucide-leptos` adoption costs are in DR-0014
- [x] No durable document depends on this log
