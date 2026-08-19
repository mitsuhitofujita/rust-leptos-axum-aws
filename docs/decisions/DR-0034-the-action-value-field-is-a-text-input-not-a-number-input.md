# DR-0034: The action value field is a text input, not a number input

Status: accepted
Date: 2026-08-19

## Context

The create-action and edit-action screens' Value field originally used
`<input type="number" ... prop:value=value_text on:input=...>`, matching the
static `docs/design/html/actions-create.html` and `actions-edit.html`
mockups those screens were built from. In practice, no user could enter a
decimal value: typing a digit then `.` never left the `.` in the box.

The cause is an interaction between two things that are each individually
correct. First, `prop:value` in this Leptos binding is a controlled input: on
every `input` event, the signal is set from `event_target_value`, and the
signal's new value is written straight back to the DOM node's `value`
property. Second, per the HTML specification, a `type="number"` input's
`value` IDL property returns the empty string whenever the box's current
text does not match the "valid floating-point number" grammar — and a bare
trailing decimal point (`"5."`) does not match it, since the grammar requires
at least one digit after the `.`. The moment a user typed the `.`,
`event_target_value` read back a value with the `.` already stripped by the
browser, and the reactive binding wrote that shorter value straight back to
the DOM, erasing the `.` before a fractional digit could ever follow it. This
is independent of locale, keyboard layout, or typing order — decimal entry
was impossible for every user.

## Decision

The Value input on both the create-action and edit-action screens uses
`type="text"` with `inputmode="decimal"`, not `type="number"`. The mobile
numeric/decimal keypad `inputmode="decimal"` still requests is unaffected;
only the browser's number-specific input handling — the sanitization
algorithm described above, plus the spinner controls and locale-dependent
`e`/`,` handling `type="number"` also carries — no longer applies. Validation
that the entered text is a finite number happens the same way it already
did: `.trim().parse::<f64>()` on submit, reporting `"A numeric value is
required."` on failure.

## Alternatives

- **Keep `type="number"`, change how the value is read.** Reading the raw
  text through a `NodeRef` instead of `event_target_value`, or switching from
  `on:input` to `on:change`, would dodge the specific keystroke-level
  round-trip that breaks today — but `on:change` only fires on blur, making
  every other screen's live-typing feedback (the unit suffix, the inline
  error) lag behind what the visitor is looking at, and a `NodeRef` read
  still inherits `type="number"`'s other quirks: scroll-wheel-driven value
  changes, silent acceptance of scientific notation (`1e2`), and the spinner
  buttons the mockup's own CSS already hides. Rejected: it treats the
  symptom rather than the input type that causes it.
- **Debounce the reactive write-back.** Delaying when `prop:value` re-applies
  the signal to the DOM would mask the immediate erasure but not fix it — the
  sanitized (shorter) value is still what ends up in the signal, so the `.`
  is still lost, just slightly later. Rejected as not actually solving the
  problem.

## Consequences

Any future numeric input added to this codebase using the same
`prop:value` + `on:input` controlled pattern must not use `type="number"` —
it should use `type="text"` with `inputmode="decimal"` (or `"numeric"` for an
integer-only field) and validate the parsed result on submit, the way this
field now does. This is not obvious from reading the HTML mockups, which use
`type="number"` throughout since they carry no reactive controlled binding
and so never exhibit the bug themselves.

This makes native browser affordances specific to `type="number"` — the
spinner buttons, up/down arrow-key stepping, scroll-wheel adjustment — no
longer available, none of which this app's mockups relied on or `main.css`
depended on (confirmed: no rule referenced `appearance` or
`::-webkit-inner-spin-button`). Reversing this decision would cost only
re-adding `type="number"`, but would reintroduce the bug for as long as the
input stays a Leptos-controlled `prop:value` binding.
