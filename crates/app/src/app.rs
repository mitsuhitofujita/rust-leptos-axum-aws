use leptos::prelude::*;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::path;

use crate::api::fetch_greeting;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <header class="site-header">
                <span class="site-title">"rust-leptos-axum-aws"</span>
                <nav>
                    <A href="/">"Home"</A>
                    <A href="/about">"About"</A>
                </nav>
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

#[component]
fn HomePage() -> impl IntoView {
    // `LocalResource` rather than `Resource`: the browser fetch future is not
    // `Send`, and in a CSR build nothing ever runs on the server.
    let greeting = LocalResource::new(fetch_greeting);

    view! {
        <h1>"Home"</h1>
        <p>"The message below is fetched from the axum API."</p>
        <Suspense fallback=|| view! { <p class="status">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match greeting.await {
                    Ok(greeting) => view! { <p class="greeting">{greeting.message}</p> }.into_any(),
                    Err(error) => view! { <p class="error">{error}</p> }.into_any(),
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
