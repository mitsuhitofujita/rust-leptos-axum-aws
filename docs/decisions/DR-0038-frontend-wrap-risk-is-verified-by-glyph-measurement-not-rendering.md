# DR-0038: Frontend wrap-risk is verified by real glyph measurement, not by rendering, in this dev container

Status: accepted
Date: 2026-08-20

## Context

Tuning the app's title `letter-spacing` (loosening it toward a more natural
degree) needed a way to check whether the change would push any existing
title from fitting on one line to wrapping onto a second — the kind of check
that is normally done by rendering the page in a browser and looking at it.

This project's dev container has no browser or graphics stack at all: no
`chromium-cli`, no Node/Playwright, no `fontconfig`, no system fonts of any
kind. The `claude-in-chrome` skill, which drives a real Chrome instance
through a browser extension, reported no extension connected in this
session. An early attempt to estimate wrap risk from a generic
average-character-width heuristic (rather than a real font) produced an
implausible result — it claimed existing, already-shipped title copy
overflowed its available width at the narrowest supported viewport, which
was not credible enough to make a design decision on.

## Decision

Wrap-risk for a text/typography change is verified by measuring real glyph
advance widths from the actual font, not by rendering the page. Concretely:
fetch the real font file (Inter's static `.ttf`, in this case obtained from
`fonts.googleapis.com`'s legacy-user-agent CSS response, which serves a
plain TTF URL instead of a `.woff2` one), and sum each candidate string's
glyph advances plus `letter-spacing` with a small Rust tool built on
`ttf-parser`, comparing the total against the element's real available width
computed from `style/main.css`'s own shell/gutter/dialog/sibling-element
values. This measures "does it still fit on one line" precisely without any
browser.

It does not answer "does it look right" — that visual judgment is left to
whoever has a real browser: normally the human developer, or an agent
session with `claude-in-chrome` actually connected.

## Alternatives

**Install a headless browser toolchain (Chromium, Playwright/Node) in the dev
container.** Would answer both the wrap-risk and the visual question at once,
and is the `run` skill's own default recommendation for a browser-driven app.
Not done: this container's minimal toolchain (no Python, no Node — see
`CLAUDE.md`) reads as a deliberate choice rather than an oversight, and
installing a browser stack is a heavier, more permanent change than one
work item's verification step justified.

**Skip automated wrap-risk verification and rely only on human visual
review.** Rejected: a precise, automatable check that needs no browser at all
is cheap once a real font file is in hand, and catching a wrap regression
before asking for a human's time is strictly better than not catching it.

**Estimate character widths with a generic average-width heuristic, no real
font.** Tried first and rejected: on the case that actually mattered — the
narrowest supported viewport (320px) — it produced a result implausible
enough (claiming already-shipped copy didn't fit) to be untrustworthy as the
basis for a decision.

## Consequences

Easy: a future CSS or typography change made from this same sandboxed
environment can get a fast, precise, real-metrics wrap-risk answer without
any browser, using a throwaway Rust + `ttf-parser` script — no new
dependency in the repo itself, since the tool lives outside it.

Hard: this technique only ever answers the mechanical fitting question. The
actual aesthetic judgment — does the tracking read as natural, does the page
look right — still requires a human, or an agent with a genuinely connected
browser, and cannot be self-certified from this container.

Cheap to reverse: nothing in the codebase encodes this decision; it is a
verification habit for this environment, not a structural choice. If a
browser stack becomes available in this container (or `claude-in-chrome`
gets connected) a future session can go back to rendering directly.
