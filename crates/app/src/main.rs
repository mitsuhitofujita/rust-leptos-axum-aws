mod action_types;
mod actions;
mod api;
mod app;
mod auth;
mod dashboard;
mod format;
mod home;
mod icon_catalog;
mod icon_picker;
mod icons;
mod type_picker;

use app::App;

fn main() {
    // Turns a Rust panic into a readable stack trace in the browser console
    // instead of an opaque "unreachable executed".
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
