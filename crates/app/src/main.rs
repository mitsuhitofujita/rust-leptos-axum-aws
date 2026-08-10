mod action_types;
mod api;
mod app;
mod auth;
mod dashboard;
mod home;
mod icon_catalog;
mod icon_picker;
mod icons;

use app::App;

fn main() {
    // Turns a Rust panic into a readable stack trace in the browser console
    // instead of an opaque "unreachable executed".
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
