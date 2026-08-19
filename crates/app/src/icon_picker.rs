//! The action-type icon field: a compact selector, and the modal it opens.
//!
//! One component rather than two, because the form has no use for either half
//! alone — and because the edit screen will want exactly this pair as well.
//!
//! The picker is native throughout, which is the whole of DR-0013: a `<dialog>`
//! shown with `showModal` keeps focus inside and closes on Escape, a text input
//! does its own editing, and a radio group does its own arrow keys. None of that
//! is reimplemented here. What is written is the filtering, the count, and the
//! rule that activating a result row applies it and closes the dialog in the
//! same action (DR-0035, narrowing DR-0013).

use leptos::html::{Button, Dialog, Input};
use leptos::prelude::*;

use crate::icon_catalog::{self, CATALOG};
use crate::icons::{ActivityGlyph, Check, Close, Glyph};

/// The `Icon` field of an action-type form.
///
/// `icon` is the form's own signal and is written exactly once per visit to the
/// picker: when a result row is activated. Closing the dialog without
/// selecting anything leaves it alone (DR-0035, narrowing DR-0013).
#[component]
pub fn IconField(icon: RwSignal<String>) -> impl IntoView {
    let dialog: NodeRef<Dialog> = NodeRef::new();
    let trigger: NodeRef<Button> = NodeRef::new();
    let search: NodeRef<Input> = NodeRef::new();

    let query = RwSignal::new(String::new());
    let expanded = RwSignal::new(false);

    // The catalog is 725 rows and each one is an SVG. Building them all into
    // the screen at first paint would be paid for by every visitor, including
    // the ones who keep the icon they were given; building them the first time
    // the dialog opens pays for it once, and only when it is wanted.
    let opened_once = RwSignal::new(false);

    // Lowercased once per keystroke rather than once per row per keystroke.
    let needle = Memo::new(move |_| query.get().trim().to_lowercase());
    let matches =
        move |entry: &icon_catalog::Icon| entry.display.to_lowercase().contains(&needle.get());
    let count = Memo::new(move |_| CATALOG.iter().filter(|entry| matches(entry)).count());

    let open = move |_| {
        query.set(String::new());
        opened_once.set(true);
        expanded.set(true);

        if let Some(element) = dialog.get_untracked() {
            let _ = element.show_modal();
        }
        // `showModal` moves focus into the dialog on its own, but not
        // necessarily here, and the search field is where DR-0013 puts it.
        if let Some(element) = search.get_untracked() {
            element.set_value("");
            let _ = element.focus();
        }
    };

    let close = move || {
        if let Some(element) = dialog.get_untracked() {
            element.close();
        }
    };

    // A row's `click`, not its `change` — a native radiogroup fires `change`
    // as arrow-key focus moves between options with no click involved, and
    // binding immediate apply-and-close to `change` would end browsing on the
    // first arrow press. `click` fires for a pointer tap and for
    // Space-activation of a focused radio, but not for arrow traversal alone
    // (DR-0035).
    let select = move |name: &'static str| {
        // Only a name this build knows, so a value from a stale row can never
        // reach the form. It should not be possible; the cost of checking is
        // a comparison.
        if icon_catalog::find(name).is_some() {
            icon.set(name.to_owned());
        }
        close();
    };

    // Everything that dismisses the dialog ends here — Escape, the close
    // control, and selecting a choice — so returning focus is written once.
    let dismissed = move |_| {
        expanded.set(false);
        if let Some(element) = trigger.get_untracked() {
            let _ = element.focus();
        }
    };

    view! {
        <div class="field">
            <span class="field-label" id="icon-field-label">"Icon"</span>
            <button
                class="icon-select"
                type="button"
                node_ref=trigger
                aria-haspopup="dialog"
                aria-controls="icon-picker-dialog"
                aria-describedby="icon-help"
                aria-expanded=move || expanded.get().to_string()
                // The glyph carries no words, so the accessible name is the
                // field's label followed by the current Lucide name (DR-0014).
                aria-labelledby="icon-field-label selected-icon-name"
                on:click=open
            >
                <span class="selected-icon" aria-hidden="true">
                    {move || view! { <ActivityGlyph icon=icon.get() /> }}
                </span>
                <span class="sr-only" id="selected-icon-name">
                    {move || display_name(&icon.get())}
                </span>
            </button>
            <p class="field-help" id="icon-help">
                "Search the Lucide library and choose one symbol."
            </p>
        </div>

        <dialog
            class="icon-dialog"
            id="icon-picker-dialog"
            node_ref=dialog
            aria-labelledby="icon-picker-title"
            on:close=dismissed
        >
            <div class="icon-dialog-header">
                <div>
                    <p class="icon-dialog-label">"Action type icon"</p>
                    <h2 id="icon-picker-title">"Choose an icon"</h2>
                </div>
                <button
                    class="icon-dialog-close"
                    type="button"
                    aria-label="Close icon picker"
                    on:click=move |_| close()
                >
                    <Close />
                </button>
            </div>

            <label class="search-field">
                <span class="field-label">"Search icons"</span>
                <input
                    class="icon-search"
                    type="search"
                    placeholder="e.g. book open"
                    autocomplete="off"
                    node_ref=search
                    on:input=move |event| query.set(event_target_value(&event))
                />
            </label>

            <p class="results-meta" aria-live="polite">
                {move || {
                    let found = count.get();
                    if found == 1 { "1 icon".to_owned() } else { format!("{found} icons") }
                }}
            </p>

            <div class="icon-results" role="radiogroup" aria-label="Available icons">
                <Show when=move || opened_once.get()>
                    {move || {
                        CATALOG
                            .iter()
                            .map(|entry| {
                                view! {
                                    <label class="icon-result" hidden=move || !matches(entry)>
                                        <input
                                            type="radio"
                                            name="picker-icon"
                                            value=entry.name
                                            prop:checked=move || icon.get() == entry.name
                                            on:click=move |_| select(entry.name)
                                        />
                                        <span class="icon-result-surface">
                                            <span class="result-icon" aria-hidden="true">
                                                <Glyph geometry=entry.geometry />
                                            </span>
                                            <span class="result-name">{entry.display}</span>
                                            <span class="result-check" aria-hidden="true">
                                                <Check />
                                            </span>
                                        </span>
                                    </label>
                                }
                            })
                            .collect_view()
                    }}
                </Show>
            </div>

            <Show when=move || count.get() == 0>
                <p class="empty-results">"No icons match your search."</p>
            </Show>
        </dialog>
    }
}

/// The official English name behind a stored one, for the selector's accessible
/// name.
///
/// A name the catalog does not know is reported as it arrived rather than
/// silently replaced: the glyph beside it is already the fallback, and saying
/// so is more use than saying nothing.
fn display_name(icon: &str) -> String {
    icon_catalog::find(icon).map_or_else(|| icon.to_owned(), |entry| entry.display.to_owned())
}
