use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::path;

use crate::auth::{self, AuthState};
use crate::dashboard::DashboardPage;
use crate::home::HomePage;

/// The auth state, shared through context so every screen reads the same one. A
/// signal rather than a resource: `complete_sign_in` runs once at mount, and
/// everything after that is a local transition.
#[derive(Clone, Copy)]
pub struct Auth(pub RwSignal<AuthState>);

/// Reads the shared auth state. Panics outside [`App`], which is the only place
/// a component can be.
pub fn auth_state() -> RwSignal<AuthState> {
    expect_context::<Auth>().0
}

#[component]
pub fn App() -> impl IntoView {
    let auth_state = RwSignal::new(AuthState::Loading);
    provide_context(Auth(auth_state));

    // Settles `Loading` into one of the other four. When the visitor is
    // returning from the hosted UI this is where the code is exchanged, so it
    // has to finish before any call that needs the token goes out — which is
    // what the dashboard's resource waits on.
    spawn_local(async move {
        auth_state.set(auth::complete_sign_in().await);
    });

    // The shell every screen shares: one mobile column, the page in it, and the
    // footer following the content rather than fixed over it. `<Routes>` renders
    // the matched screen's elements directly into `<main>`, which is what lets a
    // short screen push its primary action to the bottom with `margin-top:auto`.
    //
    // The top row belongs to the screen, not to the shell: only an authenticated
    // application screen carries the account control at its end.
    view! {
        <Router>
            <div class="app-shell">
                <main>
                    <Routes fallback=NotFound>
                        <Route path=path!("/") view=HomePage />
                        <Route path=path!("/dashboard") view=DashboardPage />
                    </Routes>
                </main>
                <footer class="site-footer">"actord"</footer>
            </div>
        </Router>
    }
}

/// The wordmark row. `children` is the account control, present only on the
/// screens that have one.
#[component]
pub fn SiteHeader(#[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <header class="site-header">
            <A href="/" attr:class="wordmark" attr:aria-label="actord home">
                // Decorative: the accessible name above already says `actord`.
                <span class="wordmark-mark" aria-hidden="true"></span>
                <span>"actord"</span>
            </A>
            {children.map(|children| children())}
        </header>
    }
}

/// Answers every path the router does not know.
///
/// Two of those paths are reachable by design rather than by mistake: a
/// dashboard row links to action creation and the account control to the
/// action-type area, and neither screen has a defined layout yet. Page Layouts
/// is explicit that they must not be inferred from the screens that do, so the
/// links point where they will eventually go and this answers until they arrive.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <SiteHeader />
        <section class="not-found">
            <p class="eyebrow">"Nothing here"</p>
            <h1>"Not found."</h1>
            <p class="lead">
                "This screen has not been built yet, or the address is wrong."
            </p>
            <A href="/" attr:class="text-link">"Back to home"</A>
        </section>
    }
}
