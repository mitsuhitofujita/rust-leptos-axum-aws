# Page Layouts

Updated: 2026-08-08

## Purpose

Define the screen inventory, information hierarchy, and navigation intent of
actord. The application records a completed action by pairing a previously
registered action type with a numeric value. An action type supplies the action
name and the unit used by that value; for example, `Running` and `km` make the
record `Running — 5.2 km`.

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

The wordmark starts the top row and returns home. Authenticated application
screens place the user image at its end; that control must provide access to the
action-type area, whether directly or through an account menu. The home screen
may keep account identity and logout inside the page body instead. The footer
follows content rather than remaining fixed over it.

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
Each row exposes the action-type icon, action name, recorded timestamp, numeric
value and unit. The row links to action creation with that action type already
selected, so recording the same activity again is one direct transition. The
plus symbol reinforces the transition but is decorative. Long action names
truncate within the row; values and units do not wrap.

The ten-day summary and the ten-record list are separate limits. The list is not
required to contain exactly one record per chart bar.

### Remaining application screens

The following screens are part of the intended product even though their exact
visual composition has not yet been defined by an HTML reference:

| Screen | Required content and behavior |
| --- | --- |
| Action types | List the registered action types and provide access to add and edit operations. |
| Add action type | Accept an English action name and the unit for its numeric value. |
| Edit action type | Edit the name and unit of an existing action type and offer deletion. |
| Actions | List recorded actions. |
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
| Action type: name and numeric unit | Action-type management, action creation, dashboard rows |
| Action record: action type, numeric value, recorded timestamp | Dashboard summary, dashboard recent list, actions list |

Navigation that is part of the current design is:

```text
signed-out home ── Google sign-in ──▶ signed-in home
signed-in home ── Open dashboard ──▶ dashboard
dashboard recent row ──────────────▶ add action (type preselected)
authenticated avatar ──────────────▶ action-type access
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

- Action types are created before their actions and define both the displayed
  name and numeric unit.
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
