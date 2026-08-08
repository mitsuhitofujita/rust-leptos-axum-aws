# Visual Design

Updated: 2026-08-08

## Purpose

Define the visual language of actord. The interface should make a private daily
recording tool feel calm, focused, and encouraging without turning routine
actions into a competitive game. It is designed for its developer's personal
use, not for collaboration, public profiles, or social comparison.

The product is English-only, light-only, and designed for one mobile column. A
red-to-pink palette gives it a distinct identity, while a scale based on the
golden ratio keeps spacing and type relationships stable.

## Structure

### Canvas and application shell

The document canvas is a muted warm gray (`#f4edef`). The application occupies
one centered column with these dimensions:

| Property | Value | Role |
| --- | --- | --- |
| Minimum viewport width | `320px` | Smallest supported mobile viewport |
| Maximum shell width | `430px` | Keeps the interface mobile-sized on wider viewports |
| Shell minimum height | `100vh`, then `100dvh` | Fills both legacy and dynamic mobile viewports |
| Horizontal content gutter | `26px` | Shared alignment line for page content and footer |
| Top-row minimum height | `42px` | Aligns the wordmark and account control |

The shell uses the palest rose background, clips decorative overflow, and has a
soft shadow against the outer canvas. On a narrow viewport it fills the width;
on a wider viewport it remains a 430-pixel mobile surface centered in the page.
There is no tablet or desktop multi-column layout.

Thin, partly clipped circular outlines in pale rose may sit behind content.
They are decorative only, must not affect layout or accessibility, and stay
subordinate to content.

### Spacing

The spacing scale is `4`, `6`, `10`, `16`, `26`, and `42` pixels. From 6 pixels
upward, each step approximates a golden-ratio progression and is also the sum
of the two preceding steps. Use the named scale rather than introducing nearby
one-off values.

| Token | Value | Typical use |
| --- | --- | --- |
| `space-2xs` | `4px` | Tight internal separation |
| `space-xs` | `6px` | Compact gaps and small control padding |
| `space-sm` | `10px` | Inline gaps and compact block padding |
| `space-md` | `16px` | Default block gap, control radius, row padding |
| `space-lg` | `26px` | Page gutter, section gap, prominent radius |
| `space-xl` | `42px` | Major section separation and square icon container |

### Color

| Token | Value | Use |
| --- | --- | --- |
| `ink` | `#29151c` | Primary text |
| `muted` | `#765f67` | Supporting copy |
| `accent` | `#e73562` | Primary actions, emphasis, and feature cards |
| `accent-dark` | `#c91f4d` | Accent text and icons on pale surfaces |
| `accent-soft` | `#ffe5ec` | Icon and avatar backgrounds |
| `rose-50` | `#fff8fa` | Application shell |
| `rose-100` | `#fff0f4` | Hover and subtle control surfaces |
| `rose-200` | `#ffd4df` | Decoration and stronger pale borders |
| `line` | `#f0dfe4` | Dividers and card outlines |
| `white` | `#ffffff` | Text on accent surfaces and raised controls |
| outer canvas | `#f4edef` | Area outside the mobile shell |

Red and pink communicate identity and hierarchy, not errors. White text is used
on the solid accent surface. Supporting text uses muted warm colors rather than
low-opacity primary text on pale surfaces. The document declares
`color-scheme: light`; no dark-mode substitution is provided.

### Typography

Use `Inter` when available, followed by the system sans-serif stack. All
interface copy is English.

| Role | Size | Treatment |
| --- | --- | --- |
| Caption | `10px` | Weight 800, widely tracked, uppercase |
| Small/supporting | `12px` | Used for eyebrow and helper copy |
| Body | `16px` | Default prose and primary controls |
| Section title | `26px` | Weight 800, tight tracking |
| Display title/number | `42px` | Weight 800, very tight tracking and line height |
| Wordmark | `20px` | Weight 800, tight tracking |

Display copy is compact and assertive; body copy stays comfortably spaced at
about `1.625` line height. Uppercase is reserved for short labels, never for
paragraphs or action names.

### Brand and recurring surfaces

The wordmark combines the lowercase name `actord` with a small rounded square
outline, rotated slightly counterclockwise, containing an accent dot. The
wordmark links home. The mark is decorative when the accessible name already
contains `actord`.

The standard page heading uses a short uppercase eyebrow with a small leading
rule, followed by a display-size title. Selected words may use the accent color.

Primary feature and summary cards use a solid accent background, white content,
a 26-pixel radius, a rose shadow, and faint circular line decoration. Ordinary
content groups use a translucent white surface with a pale border and subtler
shadow. Avatars are circular; activity glyphs sit in 42-pixel rounded-square
containers. Icons support visible labels and do not replace them.

The footer closes every page with a separator and a centered uppercase `actord`
label. It shares the page's 26-pixel horizontal alignment.

### Interaction and motion

Interactive surfaces respond in roughly 160–180 milliseconds. Hover may raise a
primary card or button by one or two pixels and strengthen its shadow; active
controls return toward the surface. Keyboard focus is always visible as a
three-pixel translucent accent outline. A whole card or record row may be the
target when it represents one action.

Sections may enter by fading in while moving upward 10 pixels over roughly
500–650 milliseconds, with short staggered delays. When
`prefers-reduced-motion: reduce` is active, animations and transitions are
reduced to effectively instantaneous durations.

## Interfaces

This document supplies the tokens and visual rules used by
[Page Layouts](page-layouts.md) and by the CSS in `style/main.css`.

The three HTML references under [`html/`](html/) show the intended application
of these rules to signed-out home, signed-in home, and dashboard states. They
are design references, not runtime entry points; the shipped SPA remains rooted
at the repository's `index.html` as described in [Frontend](frontend.md).

## Constraints

- Support only a one-column mobile composition between 320 and 430 pixels wide.
  Wider viewports center that composition instead of reflowing it.
- Use English for every user-visible label, message, date context, and control.
- Provide light mode only. Do not infer a dark palette from system preference.
- Preserve the shared spacing, type, and color tokens across screens. New values
  require a distinct semantic need, not merely a local visual adjustment.
- Keep meaningful text and controls in semantic HTML. Decorative circles,
  charts, and icon flourishes are hidden from assistive technology; informative
  images and icon-only controls receive accessible names.
- Do not rely on hover or motion to communicate state. Keyboard focus remains
  visible, and reduced-motion preference is honored.
- CSS remains plain and hand-written; the frontend introduces no CSS framework
  or Node-based styling tool, consistent with [Frontend](frontend.md).
