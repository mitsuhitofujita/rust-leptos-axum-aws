# DR-0037: A still-settling composition shows its eyebrow, not its title, and no entrance animation

Status: accepted
Date: 2026-08-20

## Context

After DR-0036, live-browser testing across three rounds showed that removing
`Loading`'s entrance animation was not enough on its own. `HomePage`'s
`Loading` composition still mounts and unmounts around the one case DR-0036
leaves asynchronous — a code exchange after the Google redirect — and with an
already-live Google session that exchange can complete fast enough that even
a purely static swap between `Loading`'s composition and the settled state's
is visible as a flash.

Three visual treatments were tried in sequence, each live-tested by the user:

1. The signed-out home's full animated heading (`<SignedOutIntro/>`) —
   visibly wrong content ("Make every action count.") appearing and then
   vanishing for an already-signed-in visitor.
2. A small `<p class="status">` caption, no heading, no animation — no wrong
   content and no motion, but a short single line growing abruptly into
   `SignedInHome`'s full heading-and-body composition still read as a flash.
3. An `<h1>` at the title's own size and position, still no animation —
   reported by the user as too visually prominent ("目立ちすぎ") for a
   composition meant to last a moment at most.

## Decision

Home's `Loading` composition renders its status copy at the eyebrow's weight
and position — `<p class="eyebrow loading-eyebrow">"Checking your
session…"</p>` — with no accompanying title and no entrance animation.
`style/main.css`'s `p.loading-eyebrow` rule supplies only `.page-heading`'s
`margin-top` (and its `max-height: 720px` contraction to `--space-md`),
deliberately omitting the `enter` animation `.page-heading` itself carries.
The selector combines the element type with the class specifically so its
`margin-top` wins over the plain `.eyebrow` rule's `margin: 0 0
var(--space-md)` shorthand by specificity, rather than by depending on which
rule happens to come later in the stylesheet.

## Alternatives

The three rejected treatments above. Also offered and not taken: hold
`Loading`'s own composition back behind a short delay-before-show timer, so a
fast settle shows nothing at all in the interval. Rejected in favor of
matching the eventual heading's own shape instead, which needs no timer and
behaves the same regardless of how fast or slow the exchange actually is.

## Consequences

Easy: any future transient or settling composition in this app has a
documented, tested shape to copy — eyebrow weight and position, no title, no
entrance animation — rather than needing to rediscover through live testing
which of several plausible options actually avoids a flash.

Hard, and accepted: `RequireAuth`'s own `Access::Pending` view (`app.rs`)
still renders a plain `.status` caption for guarded screens behind the
router, which does not follow this convention and could show the same kind
of flash if a guarded screen's own settle time is ever fast enough to
matter. Not fixed here, since the report this work answers was about home
specifically. Reversing this decision means reverting `home.rs`'s `Loading`
arm and removing `p.loading-eyebrow` from `style/main.css`.
