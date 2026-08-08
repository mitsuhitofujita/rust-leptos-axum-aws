//! Every inline SVG the interface draws.
//!
//! They live together because none of them is content: each one supports a
//! visible label or a control that already has an accessible name, so all of
//! them are `aria-hidden` at their use site and none carries text of its own.
//! Keeping them out of the screen modules is what leaves those readable as
//! layout.

use leptos::prelude::*;

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

/// The plus on a dashboard row. Decorative: the row's own text says what the
/// row does.
#[component]
pub fn Plus() -> impl IntoView {
    view! {
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M12 5v14M5 12h14" />
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

/// The glyph for an action type, chosen by [`shared::ActionType::icon`].
///
/// An identifier this does not know draws the fallback rather than nothing: the
/// icon set is the frontend's, the identifier comes over the wire, and the two
/// can disagree the moment a type is registered that predates a glyph for it.
#[component]
pub fn ActivityGlyph(icon: String) -> impl IntoView {
    match icon.as_str() {
        "running" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="m13.2 5.8-2.4 4 3.1 2.2 1.9 4.2" />
                <path d="m10.8 10-3.1 2.6-2.8-.3M13.9 12l-3.2 4.6-3.7 1.6" />
                <circle cx="15.4" cy="3.6" r="1.5" />
            </svg>
        }
        .into_any(),
        "water" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                <path d="M7 3h10v5c0 4-2 7-5 9-3-2-5-5-5-9V3Z" />
                <path d="M9 8h6" />
            </svg>
        }
        .into_any(),
        "reading" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4.5 5.5c2.8-.8 5.2-.2 7.5 1.7v11c-2.3-1.9-4.7-2.5-7.5-1.7zM19.5 5.5c-2.8-.8-5.2-.2-7.5 1.7v11c2.3-1.9 4.7-2.5 7.5-1.7z" />
            </svg>
        }
        .into_any(),
        "meditation" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <circle cx="12" cy="12" r="8" />
                <path d="M12 8v4l2.5 2" />
            </svg>
        }
        .into_any(),
        "cycling" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="6" cy="16" r="3.5" />
                <circle cx="18" cy="16" r="3.5" />
                <path d="m6 16 4-8h3l5 8M9 10h6" />
            </svg>
        }
        .into_any(),
        "strength" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <path d="M5 7v10M19 7v10M5 12h14M8 9v6M16 9v6" />
            </svg>
        }
        .into_any(),
        "study" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M5 19h14M7 16V8l5-3 5 3v8M9.5 16v-4h5v4" />
            </svg>
        }
        .into_any(),
        "walking" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <path d="M8 5c2 2 3 4 3 6s-1 4-3 6M16 5c-2 2-3 4-3 6s1 4 3 6" />
            </svg>
        }
        .into_any(),
        "sleep" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <path d="M4 16h16M7 16v-4h10v4M6 19h12" />
            </svg>
        }
        .into_any(),
        "stretching" => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <path d="M6 18c3-1 5-3 6-6 1 3 3 5 6 6M12 12V5" />
            </svg>
        }
        .into_any(),
        _ => view! {
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <circle cx="12" cy="12" r="8" />
                <circle cx="12" cy="12" r="3" />
            </svg>
        }
        .into_any(),
    }
}
