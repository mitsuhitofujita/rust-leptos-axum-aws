use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::path;

use crate::api::{ApiError, fetch_greeting};
use crate::auth::{self, AuthState};

/// The auth state, shared through context so the header and the pages read the
/// same one. A signal rather than a resource: `complete_sign_in` runs once at
/// mount, and everything after that is a local transition.
#[derive(Clone, Copy)]
struct Auth(RwSignal<AuthState>);

#[component]
pub fn App() -> impl IntoView {
    let auth_state = RwSignal::new(AuthState::Loading);
    provide_context(Auth(auth_state));

    // Settles `Loading` into one of the other four. When the visitor is
    // returning from the hosted UI this is where the code is exchanged, so it
    // has to finish before any call that needs the token goes out — which is
    // what `HomePage` waits on.
    spawn_local(async move {
        auth_state.set(auth::complete_sign_in().await);
    });

    view! {
        <Router>
            <header class="site-header">
                <span class="site-title">"rust-leptos-axum-aws"</span>
                <nav>
                    <A href="/">"Home"</A>
                    <A href="/about">"About"</A>
                </nav>
                <AuthControl />
            </header>
            <main>
                <Routes fallback=NotFound>
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/about") view=AboutPage />
                </Routes>
            </main>
        </Router>
    }
}

/// Who is signed in, and the way in or out.
///
/// Renders nothing at all in an unconfigured build: no sign-in exists to offer,
/// and the development page should look exactly as it did before this flow
/// arrived.
#[component]
fn AuthControl() -> impl IntoView {
    let auth_state = expect_context::<Auth>().0;

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
        AuthState::Disabled => ().into_any(),
        AuthState::Loading => view! { <span class="auth status">"…"</span> }.into_any(),
        AuthState::SignedOut => view! {
            <button class="auth" on:click=sign_in>"Sign in"</button>
        }
        .into_any(),
        AuthState::SignedIn { email } => view! {
            <span class="auth">
                <span class="auth-email">{email.unwrap_or_else(|| "Signed in".to_owned())}</span>
                <button on:click=sign_out>"Sign out"</button>
            </span>
        }
        .into_any(),
        // The error is reported here rather than swallowed, and the way to
        // recover is offered beside it.
        AuthState::Error(message) => view! {
            <span class="auth">
                <span class="error">{message}</span>
                <button on:click=sign_in>"Sign in"</button>
            </span>
        }
        .into_any(),
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let auth_state = expect_context::<Auth>().0;

    // `LocalResource` rather than `Resource`: the browser fetch future is not
    // `Send`, and in a CSR build nothing ever runs on the server.
    //
    // Reading the auth signal in the source closure is load-bearing. Without it
    // the first fetch after a sign-in leaves before `complete_sign_in` has
    // stored the token, renders the 401, and never retries — the state settling
    // is precisely the event that should re-run this.
    let greeting = LocalResource::new(move || {
        let _ = auth_state.get();
        fetch_greeting()
    });

    // A 401 means the token this tab holds is not one the API accepts, so it is
    // dropped and the header falls back to offering a sign-in.
    //
    // The guard is what keeps this from looping. Only a signed-in state
    // transitions; a 401 arriving when there was no token to blame writes
    // nothing, because writing would re-run the resource, fail the same way, and
    // write again.
    Effect::new(move || {
        if matches!(greeting.get(), Some(Err(ApiError::Unauthorized)))
            && matches!(auth_state.get_untracked(), AuthState::SignedIn { .. })
        {
            auth::forget_session();
            auth_state.set(AuthState::SignedOut);
        }
    });

    view! {
        <h1>"Home"</h1>
        <p>"The message below is fetched from the axum API."</p>
        <Suspense fallback=|| view! { <p class="status">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match greeting.await {
                    Ok(greeting) => view! { <p class="greeting">{greeting.message}</p> }.into_any(),
                    Err(ApiError::Unauthorized) => view! {
                        <p class="error">"The API refused this request. Sign in and try again."</p>
                    }
                    .into_any(),
                    Err(error) => view! { <p class="error">{error.to_string()}</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn AboutPage() -> impl IntoView {
    view! {
        <h1>"About"</h1>
        <p>
            "A client-side rendered Leptos single-page application, built by trunk
             and served as static files, talking to a separate axum API."
        </p>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <h1>"Not found"</h1>
        <p><A href="/">"Back to home"</A></p>
    }
}
