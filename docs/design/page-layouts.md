# Page Layouts

Updated: 2026-08-16

## Purpose

Define the screen inventory, information hierarchy, and navigation intent of
actord. The application records a completed action by pairing a previously
registered action type with a numeric value. An action type supplies an icon,
the action name, and the unit used by that value; for example, a running glyph,
`Running`, and `km` make the record `Running — 5.2 km`.

This document defines layout intent rather than routes or component ownership.
The shared visual rules are in [Visual Design](visual-design.md).

## Structure

### Shared page anatomy

Every screen is a single vertical flow inside the mobile application shell:

```text
┌──────────────────────────────────┐
│ actord                     avatar│  top row
│                                  │
│ eyebrow                          │
│ Page title                       │  page heading
│                                  │
│ page-specific content            │  scrolls when needed
│                                  │
├──────────────────────────────────┤
│              ACTORD              │  footer
└──────────────────────────────────┘
```

The wordmark starts the top row and returns home. An authenticated application
screen places the user image at its end; activating it opens an account menu
reaching the action-type area, the actions list, and signing out. The home
screen keeps account identity and logout inside the page body instead, rather
than behind the same menu. The footer follows content rather than remaining
fixed over it.

Short screens use the full dynamic viewport and can push the primary action
toward the bottom. Content-rich screens grow vertically and use normal document
scrolling. At viewport heights of 720 pixels or less, top and section gaps
contract from 26 to 16 pixels; content is not scaled down.

### Signed-out home

The signed-out home is a focused authentication landing page. From top to
bottom it contains:

1. The `actord` wordmark.
2. The eyebrow `Small actions, real progress`.
3. The message `Make every action count.` with `action count.` accented.
4. A short explanation of recording actions and the numbers that matter.
5. A full-width `Continue with Google` button placed toward the bottom.
6. The standard footer.

The Google button is a raised white surface and includes the Google mark. It is
the only primary interaction on this state.

### Signed-in home

The signed-in home preserves the landing-page rhythm but personalizes it. It
contains:

1. The `actord` wordmark.
2. A `Welcome back` eyebrow and a greeting using the user's display name.
3. Brief encouragement to continue recording.
4. An account strip with profile image, display name, email, and `Log out`.
5. A large accent card linking to the dashboard, pushed toward the bottom when
   viewport height permits.
6. The standard footer.

The account email truncates on one line rather than widening the shell. The
dashboard card is one link, with its label, message, and arrow all inside the
same target.

### Dashboard

The dashboard is the authenticated overview and repeat-recording surface:

```text
actord                          avatar

YOUR PROGRESS
Dashboard

┌ Recent ───────── ten-day bars ┐
│ total actions                 │
└───────────────────────────────┘

Recent actions        N records
Tap an action to record it again.
┌ icon  name / time  value unit + ┐
├ icon  name / time  value unit + ┤
└ ... up to ten latest records ...┘
```

The accent summary card reports the total number of actions in the recent
ten-day window and visualizes one bar per day. It communicates the total in text;
the bars are supplemental and hidden from assistive technology.

The recent list contains at most the ten latest action records, newest first.
Each row exposes the action type's configured icon, action name, recorded
timestamp, numeric value and unit. The row links to action creation with that
action type already selected, so recording the same activity again is one direct
transition. The plus symbol reinforces the transition but is decorative. Long
action names truncate within the row; values and units do not wrap.

The ten-day summary and the ten-record list are separate limits. The list is not
required to contain exactly one record per chart bar.

### Action types

The action types screen is the authenticated management index:

```text
actord                          avatar

YOUR SETUP
Action types
Supporting explanation

┌          Add action type         ┐
└──────────────────────────────────┘

Your types                       N types
Choose a type to edit its name, unit, or icon.
┌ icon name                 unit  › ┐
├ icon name                 unit  › ┤
└ ... registered action types ...  ┘
```

The full-width solid accent control opens action-type creation. The registered
types appear on an ordinary content surface and the visible count is derived
from that collection. Each row shows the configured icon, action name, and unit,
and the whole row opens that type for editing. The icon is supplemental to the
visible name and hidden from assistive technology; the chevron is decorative.
Long names truncate before displacing the unit, which remains visible on one
line, and the row carries no further supporting copy.

The screen has an empty state, because every account begins in one. It replaces
the list with a single line of copy on a dashed outline of the same shape, so
the screen does not change composition when the first type lands on it; the
`Add action type` control above is already the way out, so the copy says what is
missing and offers nothing further. The section heading and its count stay,
reading `0 types`.

The reference defines the populated state only. Loading, error, and pagination
behavior remain unspecified. The complete visual reference is
[`html/action-types-list.html`](html/action-types-list.html).

### Add action type

The add action type screen is an authenticated, single-purpose form:

```text
actord                          avatar

ACTION TYPES
Create a
new type.

┌ Action name ─────────────────────┐
│ e.g. Running                     │
│ supporting guidance              │
│                                  │
│ Numeric unit                     │
│ e.g. km                          │
│ supporting guidance              │
│                                  │
│ Icon                             │
│ ┌───────┐                        │
│ │ glyph │                        │
│ └───────┘                        │
│ supporting guidance              │
└──────────────────────────────────┘

┌         Create action type       ┐
└──────────────────────────────────┘
                Cancel
```

The fields live together on an ordinary content surface. All three are required:
the action name is the label shown on records, the numeric unit is displayed
beside every recorded value, and the icon is the supplemental glyph shown with
the type. One compact icon-only selector shows the current glyph; its field label
provides visible context and its accessible name also includes the current
Lucide name. Activating it opens the searchable modal picker described below.
The examples `Running` and `km` demonstrate the relationship without supplying
initial text values.
The solid accent button is the single primary action; `Cancel` returns to the
action types screen without saving (DR-0013). A successful creation returns
there too, so the new type is seen in the list it joined rather than announced
on the form that made it. A refused one keeps the visitor on the form and states
the reason above the primary action, in the words the service used.

On a short viewport the standard top and section gaps contract. On a taller
viewport the action group moves toward the bottom of the available shell while
remaining after the fields in document and keyboard order. The complete visual
reference is
[`html/action-types-create.html`](html/action-types-create.html).

### Edit action type

The edit action type screen uses the same field composition as creation, with
the selected type's current values filled in:

```text
actord                          avatar

ACTION TYPES
Edit action
type.

┌ Action name ─────────────────────┐
│ Running                          │
│ supporting guidance              │
│                                  │
│ Numeric unit                     │
│ km                               │
│ supporting guidance              │
│                                  │
│ Icon                             │
│ ┌───────┐                        │
│ │ glyph │                        │
│ └───────┘                        │
│ supporting guidance              │
└──────────────────────────────────┘

┌             Save changes         ┐
└──────────────────────────────────┘
                Cancel

────────────────────────────────────
Delete this action type
Supporting consequence copy
┌         Delete action type       ┐
└──────────────────────────────────┘
```

The compact selector shows only the selected type's current glyph and opens the
same searchable picker used by creation. Its accessible name includes the
current Lucide name (DR-0013, DR-0014). Save is the solid accent primary action
and `Cancel` returns without applying changes. Deletion is
separated from the routine form by a section gap and divider, explained in text,
and rendered as an outlined full-width button. Its label and trash glyph
communicate the operation without relying on color alone.

The edit reference defines placement of the delete trigger. Activating it opens
the custom confirmation state described below. The effect on existing action
records, success and error feedback, and post-delete navigation remain
unspecified. The complete edit-page visual reference is
[`html/action-types-edit.html`](html/action-types-edit.html).

### Action type icon picker

Create and edit open the same modal selection state from their compact icon
field:

```text
╔ dimmed form page ════════════════╗
║  ACTION TYPE ICON             ×  ║
║  Choose an icon                  ║
║                                  ║
║  Search icons                    ║
║  ┌ e.g. book open ────────────┐  ║
║  └────────────────────────────┘  ║
║  N icons                         ║
║  ┌ glyph  Person Standing    ✓┐  ║
║  ├ glyph  Droplets            ┤  ║
║  ├ glyph  Book Open           ┤  ║
║  └ ... filtered choices ...   ┘  ║
║                                  ║
║  ┌     Use selected icon      ┐  ║
║  └────────────────────────────┘  ║
╚══════════════════════════════════╝
```

The normal search input receives focus on open and filters the supported Lucide
catalog by each icon's official English name. A live result count and an
explicit empty state report the filter outcome. Results form a vertically
scrollable native single-select radio group: each row pairs a glyph with its
official name and keeps focus visually distinct from the checked state. The
selection does not change the form until `Use selected icon` is activated. The
submitted identifier is the corresponding canonical kebab-case Lucide name
(DR-0014).

The native modal keeps focus inside. The close button or Escape dismisses it
without applying the staged choice, then returns focus to the compact selector.
Search uses normal platform text-editing keys and radios retain native arrow-key
behavior rather than reproducing a custom grid interaction (DR-0013).

### Delete action type confirmation

Deletion confirmation is a modal state over the edit action type page:

```text
╔ dimmed, blurred edit page ═══════╗
║                                  ║
║  ┌────────────────────────────┐  ║
║  │ trash  CONFIRM DELETION    │  ║
║  │                            │  ║
║  │ Delete Running?            │  ║
║  │ consequence and warning    │  ║
║  │                            │  ║
║  │ Running                 km │  ║
║  │                            │  ║
║  │      Keep action type      │  ║
║  │     Delete action type     │  ║
║  └────────────────────────────┘  ║
║                                  ║
╚══════════════════════════════════╝
```

The underlying edit page is inert while the dialog is open. The white dialog
identifies the selected action type by name and unit, explains that it will no
longer be available for new records, and states that deletion cannot be undone.
`Keep action type` is first in document order and receives initial focus. The
confirmed deletion action uses solid ink rather than the product accent; text
and a trash glyph convey its destructive meaning without relying on color.

The reference defines the confirmation choice but not the effect on historical
records, success and error feedback, post-delete navigation, dismissal through
Escape or the dimmed layer, or focus restoration. The complete visual reference
is
[`html/action-types-delete-confirm.html`](html/action-types-delete-confirm.html).

### Remaining application screens

The following screens are part of the intended product even though their exact
visual composition has not yet been defined by an HTML reference:

| Screen | Required content and behavior |
| --- | --- |
| Actions | List recorded actions. Reached from the account menu; lands on the router's not-found fallback until it exists. |
| Add action | Select an action type and enter its numeric value; when opened from a dashboard row, preserve the preselected type. |

These screens inherit the shared shell, heading, footer, visual tokens, focus
treatment, and reduced-motion behavior. Their route paths, empty states,
validation messages, destructive-action confirmation, and detailed control
placement remain unspecified; implementation must not treat the reference home
or dashboard layouts as resolving those product decisions.

## Interfaces

The page layouts consume three categories of data:

| Data | Used by |
| --- | --- |
| Authentication state, display name, email, profile image | Both home states and the authenticated top row |
| Action type: icon identifier, name and numeric unit | Action-type management, action creation, dashboard rows |
| Action record: action type, numeric value, recorded timestamp | Dashboard summary, dashboard recent list, actions list |

Navigation that is part of the current design is:

```text
signed-out home ── Google sign-in ──▶ signed-in home
signed-in home ── Open dashboard ──▶ dashboard
dashboard recent row ──────────────▶ add action (type preselected)
authenticated avatar ───────────────▶ account menu
account menu ── Action Type ────────▶ action-type access
account menu ── Action ─────────────▶ actions (not built; not-found fallback)
account menu ── Log out ────────────▶ signed-out home
action types ───────── Add action type ──▶ add action type
add action type ───── Cancel or created ─▶ action types
action-type row ────────────────────────▶ edit action type
add or edit action type ── icon field ──▶ icon picker
icon picker ────── Use selected icon ───▶ the form, icon applied
icon picker ────── close or Escape ─────▶ the form, icon unchanged
edit action type ───── Delete action type ──▶ deletion confirmation
deletion confirmation ── Keep action type ──▶ edit action type
any application screen, unauthenticated ─▶ signed-out home
```

An unauthenticated visitor reaching an application screen is returned to the
signed-out home, which is where signing in happens. The intended destination is
not remembered; the visitor arrives at home and continues from there. Home
itself is never redirected away from, in either direction — it is one screen with
two states, not a pair of screens (DR-0011).

Authentication behavior and the current implemented router belong to
[Frontend](frontend.md); this document supplies the intended screen behavior
that future routes and components must realize.

## Constraints

- Action types are created before their actions and define the displayed icon,
  name, and numeric unit. The icon is one canonical Lucide name chosen from the
  supported catalog, never free text or an uploaded asset — DR-0012, DR-0014.
- The icon is chosen through the modal picker rather than an inline grid, so the
  form's height does not grow with the catalog — DR-0013.
- Recording an action always requires an action type and a numeric value.
- Dashboard recent actions are capped at ten and ordered newest first.
- A dashboard recent-action row repeats its type by opening action creation with
  that type selected; it does not silently create a record.
- Authentication state changes the home layout rather than producing unrelated
  visual systems.
- All screen copy and user-entered action names and units are expected in
  English.
- Screens without an HTML reference have only the requirements stated here.
  Their detailed layout requires a later design update rather than inference.
