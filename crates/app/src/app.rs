use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_navigate;
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

/// What an unexpected 401 does: drop the session and leave the move to
/// [`RequireAuth`], which sends the visitor home the moment the state changes.
///
/// The guard on the current state is what keeps this from looping. Only a
/// signed-in state transitions; a 401 arriving with no token to blame writes
/// nothing, because writing would re-run the resource that produced it, fail the
/// same way, and write again.
///
/// This does not redirect on its own. Sending the visitor to the hosted UI here
/// would loop for as long as the API returns 401 for any reason a fresh token
/// cannot fix (DR-0010).
pub fn note_unauthorized() {
    let auth_state = auth_state();
    if matches!(auth_state.get_untracked(), AuthState::SignedIn { .. }) {
        auth::forget_session();
        auth_state.set(AuthState::SignedOut);
    }
}

#[component]
pub fn App() -> impl IntoView {
    let auth_state = RwSignal::new(AuthState::Loading);
    provide_context(Auth(auth_state));

    // Settles `Loading` into one of the other four. When the visitor is
    // returning from the hosted UI this is where the code is exchanged, so it
    // has to finish before any call that needs the token goes out — which is
    // what [`RequireAuth`] holds a guarded screen back for.
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
                        // `/` is not guarded: Page Layouts describes signed-out
                        // and signed-in home as two states of one screen, so
                        // authentication changes what it renders and not where
                        // the visitor is. There is no reverse guard either — a
                        // signed-in visitor arriving here stays here.
                        <Route path=path!("/") view=HomePage />
                        <Route
                            path=path!("/dashboard")
                            view=|| view! { <RequireAuth><DashboardPage /></RequireAuth> }
                        />
                    </Routes>
                </main>
                <footer class="site-footer">"actord"</footer>
            </div>
        </Router>
    }
}

/// What [`RequireAuth`] does with a screen, which is less than what
/// [`AuthState`] distinguishes: five states, three outcomes.
///
/// Separating this from the auth state is what lets the guard hold its children
/// still. A [`Memo`] over this enum only notifies when the outcome changes, so a
/// signal write that leaves the answer alone does not rebuild the screen behind
/// it — and rebuilding a screen means rebuilding its resources and refetching.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    Allow,
    Pending,
    Deny,
}

/// Keeps a visitor off a screen that has nothing to show them.
///
/// This is a matter of experience, not of security: API Gateway's JWT authorizer
/// is the only thing enforcing anything (DR-0010). Without the guard an
/// unauthenticated visitor reaches the screen, it calls the API, and the 401
/// comes back — the guard exists so that never becomes what a visitor sees.
///
/// Two of the five auth states are the reason this is a component rather than a
/// boolean. `Loading` covers the window in which `complete_sign_in` is still
/// exchanging an authorization code, so rejecting it would eject the very
/// visitor who is in the middle of coming back. `Disabled` is a build with no
/// Cognito configuration, and rejecting it would put `just dev-web` behind a
/// sign-in that does not exist — the opposite of what DR-0008 asks an unset
/// variable to mean.
#[component]
pub fn RequireAuth(children: ChildrenFn) -> impl IntoView {
    let auth_state = auth_state();
    let access = Memo::new(move |_| match auth_state.get() {
        AuthState::SignedIn { .. } | AuthState::Disabled => Access::Allow,
        AuthState::Loading => Access::Pending,
        // `Error` goes home rather than reporting here: the home screen already
        // renders the message alongside the retry that recovers from it.
        AuthState::SignedOut | AuthState::Error(_) => Access::Deny,
    });

    // A navigation is a side effect, so it belongs in an effect rather than in
    // the render below. `replace` is the load-bearing option: the default pushes
    // a history entry, and the back button would then return to the guarded path
    // and be bounced forward again, forever.
    let navigate = use_navigate();
    Effect::new(move || {
        if access.get() == Access::Deny {
            navigate(
                "/",
                NavigateOptions {
                    replace: true,
                    ..Default::default()
                },
            );
        }
    });

    move || match access.get() {
        Access::Allow => children(),
        Access::Pending => view! {
            <SiteHeader />
            <p class="status">"Checking your session…"</p>
        }
        .into_any(),
        // The effect above is already on its way out, so this is one frame at
        // most. Rendering nothing is what keeps the pending copy from flashing
        // on a screen the visitor is being sent away from.
        Access::Deny => ().into_any(),
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
