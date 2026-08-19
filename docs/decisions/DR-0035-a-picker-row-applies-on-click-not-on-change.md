# DR-0035: A picker row applies on click, not on change
Status: accepted
Date: 2026-08-19

## Context

DR-0013 gave the action-type icon picker a searchable modal with a staged
selection: choosing a row only marked it checked, and the choice reached the
form only once a separate `Use selected icon` button was activated. DR-0013
generalized to the action-type field on the create-action form
(`type_picker::TypeField`), which stages and applies the same way behind its
own `Use selected type` button.

That two-step flow — select, then separately confirm — was requested to
become one step: activating a result should both choose it and close the
dialog, with no further tap. Both pickers share one component shape
(`frontend.md` states this explicitly), so any change here applies to both
symmetrically.

The obvious implementation is to bind the apply-and-close behavior to the
result radio's `change` event, since that already fires exactly when a
selection changes. It does not work: a native `<input type="radio">` group
fires `change` (and `input`) on the newly-focused radio as arrow-key focus
moves between options in the group, with no click involved — that is
precisely what lets arrow keys browse a native radio group at all. Binding
immediate apply-and-close to `change` would therefore close the dialog on
the very first arrow-key press, before a keyboard user could see a second
option, let alone choose one.

## Decision

A result row applies its choice, and closes the dialog, on the row's
`click` event rather than its `change` event, in both `icon_picker::IconField`
and `type_picker::TypeField`. `click` fires for a pointer tap (including a
tap anywhere in the row's `<label>`, which the browser already turns into a
click on the associated `<input>`) and for Space-activation of a focused
radio, but never for arrow-key traversal between options on its own — so a
pointer user gets the requested one-tap selection while a keyboard user can
still arrow through the full list and commit with Space (or a click) once
they land on the wanted row.

Both components' `staged` signal and their `apply`/`cannot_apply` logic are
removed along with the two named buttons: once a click writes the choice
straight into the form's own signal, there is nothing left to stage, and
`checked` compares directly against that real signal instead of a
short-lived copy. The three static HTML mockups this behavior was originally
built from (`action-types-create.html`, `action-types-edit.html`,
`actions-create.html`, under `docs/design/html/`) had their own
independent vanilla-JS implementation of the same staged-apply pattern,
updated the same way for the same reason, so they continue to match what
they are cited as a reference for.

DR-0013's other reasoning — the native `<dialog>` for focus containment and
Escape-to-close, the plain search input, the native radiogroup and its
arrow-key behavior, and the WAI-ARIA citations behind choosing a modal
picker over an inline grid — is unaffected and still holds. Only its
staged-then-explicit-apply claim is superseded by this record.

## Alternatives

- **Bind apply-and-close to the radiogroup's `change` event.** The natural
  first choice, since `change` already means "the selection changed." Rejected
  because a native radiogroup fires `change` on arrow-key navigation between
  options, not only on an explicit choice — binding to it would end keyboard
  browsing after a single arrow press, making every option past the first
  keyboard-unreachable.
- **Keep the staged value and the confirm button, but auto-activate the
  button on `change`.** Would reach the same outward one-tap behavior for a
  pointer user while reusing the existing staged/apply plumbing, but still
  breaks keyboard arrow-key browsing for the same reason as the `change`
  alternative above — the problem is which event drives the apply, not
  whether a staged intermediate value exists.
- **Leave the confirm button for keyboard users, one-tap only for pointer
  users.** Two different interaction models for the same control were judged
  more confusing than one, and harder to keep the two mockups' and two
  components' behavior mirrored exactly, which `frontend.md` treats as a
  standing property worth preserving.

## Consequences

Choosing an icon or an action type is now one action for a pointer or touch
user, which is what was asked for, without giving up arrow-key browsing for
a keyboard user — Space or a click still commits a choice explicitly, just
without a separate named button to reach first. Both components lose their
staged-value indirection entirely, which is a real simplification (roughly
15–20 fewer lines each) rather than only a UI change. The two `apply-*`
CSS rules in `style/main.css` and their per-mockup duplicates are gone, since
nothing renders that button anymore.

The click-vs-change distinction this record turns on is a native-HTML detail
with no unit test in this project (`crates/app` has no DOM-testing setup —
`testing.md` already names this gap) and no browser available in this
devcontainer to check it in (`workspace.md`); the reasoning above must be
confirmed by hand in a real browser before this behavior is trusted, the
same open item DR-0013's own live-browser verification left for its author.
Reversing this decision — reintroducing a staged value and a confirm
button — is cheap: the removed code is a small, self-contained diff away,
not an architectural change.
