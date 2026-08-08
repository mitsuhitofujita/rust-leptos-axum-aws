mod api;
mod app;
mod auth;

use app::App;

fn main() {
    // Turns a Rust panic into a readable stack trace in the browser console
    // instead of an opaque "unreachable executed".
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
