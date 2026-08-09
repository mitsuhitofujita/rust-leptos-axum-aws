# DR-0013: Action type icons use a searchable modal picker
Status: accepted
Date: 2026-08-09

## Context

DR-0012 established an application-owned icon set and initially presented all
choices as an inline radio grid. That layout is direct for eight choices but
grows the form and becomes progressively harder to scan as the library expands.
The icon still needs a visual preview, so a plain text field is inappropriate,
and native selects do not consistently render SVG choices.

The WAI-ARIA Authoring Practices recommend type-ahead for lists with more than
seven options and define dialog popups as a supported way to choose a discrete
value. Their modal-dialog pattern requires focus to move inside, remain there
until dismissal, close with Escape, and return to the invoking control
afterward. See the official [Listbox](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/),
[Combobox](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/), and
[Modal Dialog](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) patterns.

## Decision

An action type continues to store one stable identifier from the built-in icon
set. Create and edit forms show one compact selector containing the current icon
and its text name. Activating it opens a modal picker rather than expanding all
choices in the form.

The picker puts focus in a normal search input, filters by the visible icon
names, announces the result count, and presents the matches as a vertically
scrollable single-select radio list. Each row pairs its glyph with a text name
and exposes a distinct checked state. Selection is staged until the user chooses
`Use selected icon`; closing or pressing Escape preserves the former value and
returns focus to the selector.

Native text-input editing, radio-keyboard behavior, and `<dialog>` focus
containment are retained rather than recreated with custom key handlers.

## Alternatives

- **Keep the inline grid.** Rejected because its height and scan cost grow with
  every icon, which is the problem this decision resolves.
- **Use a native select.** Rejected because it scales textually but cannot show
  the SVG preview consistently across browsers and operating systems.
- **Build an ARIA combobox with a grid popup.** Rejected because its custom
  active-descendant and two-dimensional arrow-key behavior are unnecessary when
  a mobile modal can use native search, radios, and dialog semantics.
- **Accept free text.** Rejected for the identifier-validity and compatibility
  reasons preserved from DR-0012.

## Consequences

The form stays compact no matter how many icons exist, while search and visible
names keep a larger library navigable. The picker requires a modal and filtering
logic, but those pieces use native primitives with familiar keyboard behavior.
The list can later add grouping or lazy rendering without changing the stored
identifier or the form's selected-value field. A usable icon name is now part of
every built-in icon's compatibility contract alongside its identifier and SVG.
