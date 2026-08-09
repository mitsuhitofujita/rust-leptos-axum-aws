# DR-0012: Action types choose icons from a built-in set
Status: superseded by DR-0014
Date: 2026-08-09

## Context

An action type needs an icon in addition to its name and numeric unit. The icon
appears in compact rows on the action types index and dashboard, where stable
geometry and immediate visual recognition matter. The application is a
client-side mobile interface with no media-upload or asset-management surface.

Accepting image files would introduce storage, validation, cropping, broken
asset, and accessibility concerns unrelated to recording actions. Accepting raw
SVG or arbitrary icon names would expose implementation details and make the
visual system inconsistent.

## Decision

An action type stores one stable icon identifier chosen from an
application-owned built-in set. Create and edit screens expose that set as one
required radio group. The frontend owns the corresponding SVG paths and renders
the same icon for that identifier wherever the action type appears.

The picker gives every option an accessible text name. A rendered row may hide
the icon from assistive technology when the adjacent action-type name supplies
the same identity; the icon is supplemental there.

## Alternatives

- **Uploaded raster images.** Rejected because they require a media lifecycle
  and make small monochrome activity glyphs visually inconsistent.
- **User-supplied SVG.** Rejected because sanitizing active vector content and
  preserving a coherent stroke style add risk without helping the recording
  task.
- **A free-text icon name.** Rejected because invalid or obsolete names would
  create broken presentation and expose the chosen icon library as user-facing
  syntax.
- **No configurable icon.** Rejected because the requested icon must distinguish
  action types in the list and dashboard rather than repeat one generic glyph.

## Consequences

The persisted value is small, predictable, and independent of asset URLs. Lists
and dashboards can render icons without another request, and all glyphs share
the existing visual language. Adding or retiring choices requires maintaining
identifier compatibility. Users cannot supply a symbol outside the built-in
set without a later decision that adds a safe extension mechanism.
