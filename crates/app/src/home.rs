//! The home screen: one route, two compositions.
//!
//! Page Layouts describes signed-out and signed-in home as two states of one
//! screen rather than two screens, so authentication changes what `/` renders
//! and not where the visitor is.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::app::{SiteHeader, auth_state};
use crate::auth::{self, AuthState};
use crate::icons::{ArrowRight, AvatarFallback, GoogleMark};

#[component]
pub fn HomePage() -> impl IntoView {
    let auth_state = auth_state();

    let sign_in = move |_| {
        if let Err(error) = auth::begin_sign_in() {
            auth_state.set(AuthState::Error(error));
        }
    };
    let sign_out = move |_| {
        if let Err(error) = auth::sign_out() {
            auth_state.set(AuthState::Error(error));
        }
    };

    move || match auth_state.get() {
        AuthState::SignedIn { name, picture } => {
            view! { <SignedInHome name=name picture=picture sign_out=sign_out /> }.into_any()
        }
        // `Disabled` is a build with no Cognito configuration. The signed-out
        // home is the only composition that makes sense without an identity,
        // and rendering it is what keeps `just dev-web` usable with none.
        AuthState::Disabled | AuthState::SignedOut => {
            view! { <SignedOutHome error=None sign_in=sign_in /> }.into_any()
        }
        // Reported where the retry is, rather than swallowed: signing in again
        // is what recovers from every failure this state carries.
        AuthState::Error(message) => {
            view! { <SignedOutHome error=Some(message) sign_in=sign_in /> }.into_any()
        }
        // Reached only while `complete_sign_in` is exchanging an authorization
        // code for tokens — `auth::initial_state` already resolves every other
        // case before this component's first render, so an ordinary visit with
        // an existing (or absent) session never passes through this arm.
        //
        // The status text takes the eyebrow's position and weight rather than
        // the title's: a returning visitor whose Google session is still live
        // can have the exchange settle fast enough that this composition is
        // visible for a moment at most, and an `<h1>`-sized placeholder there
        // read as more prominent than a moment-long state should be. The
        // eyebrow's slot still sits where every settled state's own eyebrow
        // sits, so the swap is a smaller jump than a bare caption line was.
        AuthState::Loading => view! {
            <SiteHeader />
            <p class="eyebrow loading-eyebrow">"Checking your session…"</p>
        }
        .into_any(),
    }
}

/// The landing copy, shared by every state that is not signed in.
#[component]
fn SignedOutIntro() -> impl IntoView {
    view! {
        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Small actions, real progress"</p>
            <h1 id="page-title">"Make every" <br /> <em>"action count."</em></h1>
            <p class="lead">
                "Record the things you do, add the numbers that matter, and watch
                 your everyday effort take shape."
            </p>
        </section>
    }
}

/// A focused authentication landing page. The Google button is the only primary
/// interaction on it.
#[component]
fn SignedOutHome(
    error: Option<String>,
    sign_in: impl FnMut(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <SiteHeader />
        <SignedOutIntro />
        <section class="auth-panel" aria-label="Sign in">
            {error.map(|message| view! { <p class="error-message">{message}</p> })}
            <button class="google-button" type="button" on:click=sign_in>
                <GoogleMark />
                <span>"Continue with Google"</span>
            </button>
        </section>
    }
}

/// The same rhythm, personalized. The account strip carries the profile image,
/// the display name and `Log out` — and no address: nothing on screen
/// identifies the account by email.
#[component]
fn SignedInHome(
    name: Option<String>,
    picture: Option<String>,
    sign_out: impl FnMut(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    // A Google account is not obliged to carry a `name` claim, and a build
    // whose pool does not map one has none either. Greeting the visitor
    // anonymously beats greeting an empty string.
    let greeting = name.clone().unwrap_or_else(|| "there".to_owned());
    let account_name = name.unwrap_or_else(|| "Signed in".to_owned());

    view! {
        <SiteHeader />

        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Welcome back"</p>
            <h1 id="page-title">"Hello," <br /> <em>{greeting}"."</em></h1>
            <p class="lead">
                "Your everyday actions are yours to shape. Keep showing up, one
                 record at a time."
            </p>
        </section>

        <section class="account" aria-label="Signed-in account">
            <span class="account-image">
                // The image repeats the name beside it, so it is decorative
                // here rather than informative.
                <ProfileImage picture=picture />
            </span>
            <span class="account-copy">
                <span class="account-name">{account_name}</span>
            </span>
            <button class="logout-button" type="button" on:click=sign_out>"Log out"</button>
        </section>

        <section class="home-action" aria-label="Dashboard">
            // One link: label, message and arrow are all inside the target.
            <A href="/dashboard" attr:class="dashboard-card">
                <p class="card-label">"Your space"</p>
                <h2>"See every action in one place."</h2>
                <span class="card-link">
                    <span>"Open dashboard"</span>
                    <span class="arrow" aria-hidden="true"><ArrowRight /></span>
                </span>
            </A>
        </section>
    }
}

/// The account's image, or a stand-in when the id token carried no `picture`.
#[component]
pub fn ProfileImage(picture: Option<String>) -> impl IntoView {
    match picture {
        Some(url) => view! { <img class="profile-image" src=url alt="" /> }.into_any(),
        None => view! { <AvatarFallback /> }.into_any(),
    }
}
