//! The action-type icon field: a compact selector, and the modal it opens.
//!
//! One component rather than two, because the form has no use for either half
//! alone — and because the edit screen will want exactly this pair as well.
//!
//! The picker is native throughout, which is the whole of DR-0013: a `<dialog>`
//! shown with `showModal` keeps focus inside and closes on Escape, a text input
//! does its own editing, and a radio group does its own arrow keys. None of that
//! is reimplemented here. What is written is the filtering, the count, and the
//! rule that a choice does not reach the form until it is applied.

use leptos::html::{Button, Dialog, Input};
use leptos::prelude::*;

use crate::icon_catalog::{self, CATALOG};
use crate::icons::{ActivityGlyph, Check, Close, Glyph};

/// The `Icon` field of an action-type form.
///
/// `icon` is the form's own signal and is written exactly once per visit to the
/// picker: when `Use selected icon` is activated. Closing the dialog any other
/// way leaves it alone (DR-0013).
#[component]
pub fn IconField(icon: RwSignal<String>) -> impl IntoView {
    let dialog: NodeRef<Dialog> = NodeRef::new();
    let trigger: NodeRef<Button> = NodeRef::new();
    let search: NodeRef<Input> = NodeRef::new();

    // What the visitor has picked but not yet applied. Separate from `icon`
    // precisely so that dismissing the dialog can discard it.
    let staged = RwSignal::new(String::new());
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
        staged.set(icon.get_untracked());
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

    let apply = move |_| {
        let choice = staged.get_untracked();
        // Only a name this build knows, so a stale staged value cannot become a
        // stored one. It should not be possible; the cost of checking is a
        // comparison.
        if icon_catalog::find(&choice).is_some() {
            icon.set(choice);
        }
        close();
    };

    // Everything that dismisses the dialog ends here — Escape, the close
    // control, and applying a choice — so returning focus is written once.
    let dismissed = move |_| {
        expanded.set(false);
        if let Some(element) = trigger.get_untracked() {
            let _ = element.focus();
        }
    };

    // Disabled when the staged choice is not among the rows on screen, so that
    // applying can never do something the visitor cannot see.
    let cannot_apply =
        move || icon_catalog::find(&staged.get()).is_none_or(|entry| !matches(entry));

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
                                            prop:checked=move || staged.get() == entry.name
                                            on:change=move |_| staged.set(entry.name.to_owned())
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

            <button
                class="apply-icon-button"
                type="button"
                disabled=cannot_apply
                on:click=apply
            >
                "Use selected icon"
            </button>
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
