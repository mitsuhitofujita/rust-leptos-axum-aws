# Page Layouts

Updated: 2026-08-20

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

### Home, settling

Home renders a third, transient composition while authentication is still
settling. It is reached only during the Google sign-in redirect's token
exchange — not on an ordinary load, which is always already resolved by the
first render (DR-0036):

1. The `actord` wordmark.
2. A status eyebrow, `Checking your session…`, at the same position and
   weight the settled compositions' own eyebrow takes.

No title accompanies it and it carries no entrance animation, so a fast
settle replaces the eyebrow's text in place rather than growing a caption or
a full heading out of it (DR-0037).

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
╚══════════════════════════════════╝
```

The normal search input receives focus on open and filters the supported Lucide
catalog by each icon's official English name. A live result count and an
explicit empty state report the filter outcome. Results form a vertically
scrollable native single-select radio group: each row pairs a glyph with its
official name and keeps focus visually distinct from the checked state.
Activating a row applies it immediately and closes the dialog — there is no
separate confirm step (DR-0035, narrowing DR-0013). The submitted identifier is
the corresponding canonical kebab-case Lucide name (DR-0014).

The native modal keeps focus inside. The close button or Escape dismisses it
without changing the current selection, then returns focus to the compact
selector. Search uses normal platform text-editing keys; a row applies on a
pointer tap or on Space-activation of a focused radio, but arrow-key traversal
between rows does not by itself apply one, so keyboard browsing is not cut
short (DR-0035).

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

### Actions

The actions screen is the authenticated history: every action record the
account has, newest first.

```text
actord                          avatar

YOUR HISTORY
Actions
Every action you've recorded, newest first.

┌            Add action            ┐
└──────────────────────────────────┘

Your actions                     N records
Tap a record to edit its value or delete it.
┌ icon  name / time      value unit › ┐
├ icon  name / time      value unit › ┤
└ ... every recorded action ...       ┘
```

The full-width solid accent control opens action creation. Each row shows the
record's action type icon, name, recorded timestamp, numeric value and unit,
and the whole row opens that record for editing — unlike a dashboard row,
which opens creation with the type preselected instead. The icon is
supplemental to the visible name and hidden from assistive technology; the
chevron is decorative, matching the action-types index's treatment of its own
rows. Long action names truncate before displacing the value and unit, which
remain visible on one line.

The list carries no page or count limit, unlike the dashboard's ten-record
cap: every recorded action appears. The screen has an empty state, because
every account begins in one before its first record — the same dashed-outline
treatment the action-types index uses, so the screen does not change
composition when the first record lands on it.

The reference defines the populated state only. Loading, error, and
pagination behavior remain unspecified. The complete visual reference is
[`html/actions-list.html`](html/actions-list.html).

### Add action

The add action screen is an authenticated, single-purpose form recording one
action against a registered type.

```text
actord                          avatar

ACTIONS
Record a
new action.

┌ Action type ──────────────────────┐
│ icon  Running               km  › │
│ supporting guidance                │
│                                     │
│ Value                              │
│ e.g. 5.2                     km    │
│ supporting guidance                │
└─────────────────────────────────────┘

┌         Record action            ┐
└──────────────────────────────────┘
                Cancel
```

Both fields live together on an ordinary content surface. The action type
field is a compact selector showing the current choice's icon, name and
unit; activating it opens a searchable modal picker choosing among the
account's own registered types — the same compact-selector-plus-modal-picker
shape the action-type icon field uses (DR-0013), generalized from choosing an
icon to choosing a type, including the rule that activating a result applies
it and closes the dialog immediately (DR-0035). The value field accepts a
number and displays the selected type's unit as a fixed suffix, which
updates when the type changes.

When opened from a dashboard row, the type is preselected rather than
defaulting to the account's first registered type — the transition page
layouts already requires for repeating an action. The solid accent button is
the single primary action; `Cancel` returns to the actions screen without
saving. A successful creation returns there too, so the new record is seen in
the history it joined rather than announced on the form that made it. A
refused one keeps the visitor on the form and states the reason above the
primary action, in the words the service used.

An account with no registered action type cannot record anything: the screen
replaces the form with a short message pointing to action-type creation
instead, since a value has nothing to be recorded against otherwise.

The complete visual reference is
[`html/actions-create.html`](html/actions-create.html).

### Edit action

The edit action screen shows the record's type and recorded time as
read-only, with only the numeric value open to correction.

```text
actord                          avatar

ACTIONS
Edit
action.

┌ Action type ───────────────────────┐
│ icon  Running                 km   │
│ supporting guidance                 │
│                                      │
│ Recorded                            │
│ 2026-08-08 07:12                    │
│                                      │
│ Value                               │
│ 5.2                             km  │
│ supporting guidance                  │
└──────────────────────────────────────┘

┌            Save changes           ┐
└────────────────────────────────────┘
                Cancel

────────────────────────────────────
Delete this action
Supporting consequence copy
┌            Delete action          ┐
└────────────────────────────────────┘
```

The type is fixed once a record is created (DR-0016), and nothing in this
design lets the recorded time change either, so both are shown rather than
offered as fields. `Save changes` is the solid accent primary action and
`Cancel` returns without applying changes. Deletion is separated from the
routine form by a section gap and divider, explained in text, and rendered as
an outlined full-width button — the same treatment the action-type edit
screen gives its own deletion trigger. Activating it opens the confirmation
state described below.

The edit reference defines placement of the delete trigger. Success and error
feedback and post-save or post-delete navigation beyond returning to the
actions screen remain unspecified. The complete visual reference is
[`html/actions-edit.html`](html/actions-edit.html).

### Delete action confirmation

Deletion confirmation is a modal state over the edit action page, matching
the action-type deletion confirmation's shape with a record's fuller summary
in place of a type's name and unit.

```text
╔ dimmed, blurred edit page ═══════╗
║                                  ║
║  ┌────────────────────────────┐  ║
║  │ trash  CONFIRM DELETION    │  ║
║  │                            │  ║
║  │ Delete this record?       │  ║
║  │ consequence and warning    │  ║
║  │                            │  ║
║  │ icon  Running       5.2 km │  ║
║  │       2026-08-08 07:12    │  ║
║  │                            │  ║
║  │       Keep action          │  ║
║  │      Delete action         │  ║
║  └────────────────────────────┘  ║
║                                  ║
╚══════════════════════════════════╝
```

The underlying edit page is inert while the dialog is open. The white dialog
identifies the selected record by its action type's icon and name, its
recorded time, and its numeric value and unit, explains that deletion cannot
be undone, and states nothing else reads or restores it. `Keep action` is
first in document order and receives initial focus. The confirmed deletion
action uses solid ink rather than the product accent, matching the
action-type version's reasoning: text and a trash glyph convey its
destructive meaning without relying on color alone.

The reference defines the confirmation choice but not success and error
feedback, post-delete navigation, dismissal through Escape or the dimmed
layer, or focus restoration. The complete visual reference is
[`html/actions-delete-confirm.html`](html/actions-delete-confirm.html).

## Interfaces

The page layouts consume three categories of data:

| Data | Used by |
| --- | --- |
| Authentication state, display name, email, profile image | Both home states and the authenticated top row |
| Action type: icon identifier, name and numeric unit | Action-type management, action creation, action editing (read-only), dashboard rows |
| Action record: action type, numeric value, recorded timestamp | Dashboard summary, dashboard recent list, actions list, action editing and its deletion confirmation |

Navigation that is part of the current design is:

```text
signed-out home ── Google sign-in ──▶ signed-in home
signed-in home ── Open dashboard ──▶ dashboard
dashboard recent row ──────────────▶ add action (type preselected)
authenticated avatar ───────────────▶ account menu
account menu ── Dashboard ───────────▶ dashboard
account menu ── Action Type ────────▶ action-type access
account menu ── Action ─────────────▶ actions
account menu ── Log out ────────────▶ signed-out home
action types ───────── Add action type ──▶ add action type
add action type ───── Cancel or created ─▶ action types
action-type row ────────────────────────▶ edit action type
add or edit action type ── icon field ──▶ icon picker
icon picker ────────── select a result ─▶ the form, icon applied
icon picker ────── close or Escape ─────▶ the form, icon unchanged
edit action type ───── Delete action type ──▶ deletion confirmation (type)
deletion confirmation (type) ── Keep action type ──▶ edit action type
actions ─────────────────── Add action ──▶ add action
add action ──────────── Cancel or created ─▶ actions
action record row ───────────────────────▶ edit action
add action ── action type field ────────▶ type picker
type picker ────────── select a result ─▶ the form, type applied
type picker ────── close or Escape ─────▶ the form, type unchanged
edit action ─────────────── Delete action ──▶ deletion confirmation (action)
deletion confirmation (action) ── Keep action ──▶ edit action
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
- Dashboard recent actions are capped at ten and ordered newest first. The
  actions screen carries no such cap: every recorded action appears, also
  newest first, since it is the account's history rather than a summary.
- A dashboard recent-action row repeats its type by opening action creation with
  that type selected; it does not silently create a record.
- Once created, an action record's type and recorded time are fixed. Editing
  changes only the numeric value — DR-0016.
- Authentication state changes the home layout rather than producing unrelated
  visual systems.
- All screen copy and user-entered action names and units are expected in
  English.
- Screens without an HTML reference have only the requirements stated here.
  Their detailed layout requires a later design update rather than inference.
