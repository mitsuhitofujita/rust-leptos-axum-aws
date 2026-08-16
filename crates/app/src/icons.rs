//! Every inline SVG the interface draws that is not an action-type icon.
//!
//! They live together because none of them is content: each one supports a
//! visible label or a control that already has an accessible name, so all of
//! them are `aria-hidden` at their use site and none carries text of its own.
//! Keeping them out of the screen modules is what leaves those readable as
//! layout.
//!
//! The chrome below is hand-written and the action-type glyphs are not: those
//! come from [`crate::icon_catalog`], which is generated from Lucide (DR-0014).
//! Drawing a chevron from the same source would mean enabling a whole further
//! category of icons nobody may choose, for one path.

use leptos::prelude::*;

use crate::icon_catalog;

/// The Google mark on the sign-in button. The only multicolour glyph here, and
/// the only one whose colours are not the palette's — they are Google's.
#[component]
pub fn GoogleMark() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" aria-hidden="true">
            <path
                fill="#4285F4"
                d="M21.6 12.23c0-.71-.06-1.4-.18-2.07H12v3.91h5.38a4.6 4.6 0 0 1-2 3.02v2.54h3.25c1.9-1.75 2.97-4.33 2.97-7.4Z"
            />
            <path
                fill="#34A853"
                d="M12 22c2.71 0 4.99-.9 6.65-2.43l-3.25-2.54c-.9.6-2.05.96-3.4.96-2.62 0-4.84-1.77-5.63-4.15H3.01v2.62A10.04 10.04 0 0 0 12 22Z"
            />
            <path
                fill="#FBBC05"
                d="M6.37 13.84A6.02 6.02 0 0 1 6.06 12c0-.64.11-1.26.31-1.84V7.54H3.01A10.04 10.04 0 0 0 2 12c0 1.61.39 3.14 1.01 4.46l3.36-2.62Z"
            />
            <path
                fill="#EA4335"
                d="M12 6.01c1.47 0 2.79.51 3.83 1.5l2.88-2.88A9.65 9.65 0 0 0 12 2a10.04 10.04 0 0 0-8.99 5.54l3.36 2.62C7.16 7.78 9.38 6.01 12 6.01Z"
            />
        </svg>
    }
}

/// The arrow in the dashboard card's link row.
#[component]
pub fn ArrowRight() -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M5 12h14M13 6l6 6-6 6" />
        </svg>
    }
}

/// The plus on a dashboard row, and on the action-type list's add control.
/// Decorative: the text beside it says what it does.
#[component]
pub fn Plus() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M12 5v14M5 12h14" />
        </svg>
    }
}

/// The chevron ending an action-type row, which opens that type for editing.
#[component]
pub fn ChevronRight() -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="m9 6 6 6-6 6" />
        </svg>
    }
}

/// The icon picker's close control.
#[component]
pub fn Close() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M6 6l12 12M18 6 6 18" />
        </svg>
    }
}

/// The tick on the icon picker's chosen row.
#[component]
pub fn Check() -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="m6 12 4 4 8-9" />
        </svg>
    }
}

/// The tick on the edit screen's "Save changes" button. Distinct from
/// [`Check`], which is sized for the icon picker's small circular badge rather
/// than for a full-width button.
#[component]
pub fn Checkmark() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m5 12 4 4L19 6" />
        </svg>
    }
}

/// The delete trigger on the edit screen, and the confirmation dialog it opens.
#[component]
pub fn Trash() -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
        </svg>
    }
}

/// The account menu's `Action` entry.
#[component]
pub fn Pulse() -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
        </svg>
    }
}

/// The account menu's `Action Type` entry.
#[component]
pub fn Tag() -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z" />
            <circle cx="7.5" cy="7.5" r=".5" fill="currentColor" />
        </svg>
    }
}

/// The account menu's `Log out` entry.
#[component]
pub fn LogOut() -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
            <polyline points="16 17 21 12 16 7" />
            <line x1="21" x2="9" y1="12" y2="12" />
        </svg>
    }
}

/// Stands in for a profile image when the id token carried no `picture` claim,
/// and in an unconfigured build, where there is no account at all.
#[component]
pub fn AvatarFallback() -> impl IntoView {
    view! {
        <svg class="profile-image" viewBox="0 0 48 48" aria-hidden="true">
            <rect width="48" height="48" fill="#ffdce5" />
            <circle cx="24" cy="19" r="8" fill="#c94a69" />
            <path d="M9 46c1.6-9.6 7.2-14.4 15-14.4S37.4 36.4 39 46" fill="#c94a69" />
        </svg>
    }
}

/// The `<svg>` every catalog icon is drawn in.
///
/// The wrapper is written once here and the geometry comes from
/// [`crate::icon_catalog`], which carries only what is inside it. These
/// attributes are Lucide's own defaults, minus the `width` and `height` the
/// stylesheet sets at each use site.
///
/// `inner_html` is what puts generated markup into an element, and it is what
/// it sounds like. Nothing user-supplied reaches it: the string is a literal in
/// a generated file, produced at build time from a pinned crate, and the name
/// that selects it has already had to match a catalog entry exactly.
#[component]
pub fn Glyph(geometry: &'static str) -> impl IntoView {
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            inner_html=geometry
        ></svg>
    }
}

/// The glyph for an action type, chosen by [`shared::ActionType::icon`].
///
/// A name the catalog does not know draws the fallback rather than nothing: the
/// catalog belongs to this build, the name arrives over the wire, and the two
/// can disagree the moment a stored type outlives the category that admitted
/// its icon (DR-0014).
#[component]
pub fn ActivityGlyph(#[prop(into)] icon: String) -> impl IntoView {
    match icon_catalog::find(&icon) {
        Some(entry) => view! { <Glyph geometry=entry.geometry /> }.into_any(),
        None => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <circle cx="12" cy="12" r="8" />
                <circle cx="12" cy="12" r="3" />
            </svg>
        }
        .into_any(),
    }
}
