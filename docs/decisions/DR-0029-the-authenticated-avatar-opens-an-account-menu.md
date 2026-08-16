# DR-0029: The authenticated avatar opens an account menu, not a direct link

Status: accepted
Date: 2026-08-16

## Context

`AccountControl`, the control every authenticated screen carries at the end of
its top row, was a direct `<A href="/action-types">`. Its doc comment reasoned
about the choice explicitly: "Page Layouts requires it to reach the
action-type area, and it is the only route to that area from the dashboard, so
it links there directly rather than opening a menu with one entry."

`docs/design/page-layouts.md` never required the direct link specifically —
it already allowed either shape ("that control must provide access to the
action-type area, whether directly or through an account menu"). The direct
link was a choice made in code, not a constraint the design imposed, and the
person who owns this project says it was never a decision they made.

A new design reference, `docs/design/html/dashboard-navigation.html`, shows
the avatar opening a menu that slides in from the right, holding four
entries: `Action` (the not-yet-built actions list), `Action Type`, a
separator, and `Log out`. The direct link's own justification — a menu is not
worth opening for one entry — no longer holds once the control has to reach
three destinations instead of one.

## Decision

The avatar opens an account menu instead of linking anywhere directly. The
menu is a native `<dialog>`, shown with `showModal()` and dismissed with
`close()`, styled the same way the existing delete-confirmation dialog and
`IconField`'s icon picker are: focus containment and Escape-to-close come
from the browser, not from code written for this menu specifically. It holds,
in order: `Action` (`/actions` — no route exists yet, so this lands on the
router's `NotFound` fallback by design, the same way a dashboard row's repeat
link already does), `Action Type` (`/action-types`), a separator, and
`Log out` (the existing `auth::sign_out()`).

## Alternatives

**Keep the direct link, and put `Action` and `Log out` somewhere else** — for
instance inside the dashboard body, the way the signed-in home keeps its own
account strip. Rejected: it would scatter account-related actions across
different screens instead of giving every authenticated screen one consistent
entry point, and the signed-in home's in-body pattern exists because that
screen is not behind the shared top-row control in the first place.

**Drop `Action` from the menu and keep it to two entries, `Action Type` and
`Log out`.** Rejected: the requested menu names `Action` explicitly as a
destination, and page-layouts.md already tracks the actions list as an
intended screen reachable from here — its absence from the menu would just
move the same "not built yet" gap somewhere less visible.

## Consequences

This gives every authenticated screen one place to reach the action-type
area, the actions list, and signing out, and any account-related destination
added later joins the same menu rather than needing a new home invented for
it.

It costs the action-type area a tap: what used to be one direct tap from the
avatar is now open-the-menu-then-tap. `Log out` also moves off the home
screen's in-body pattern for every screen behind this control, so a
authenticated visitor now has two different places `Log out` can appear
depending on which screen they are on — the account strip on `/`, or this
menu everywhere else — which page-layouts.md now states as intended rather
than as an inconsistency.

Reversing this is cheap on its own — dropping the dialog and going back to
`<A href="/action-types">` is a small change — but it re-opens the question
of where `Action` and `Log out` live once the menu that was giving them a
home is gone.
