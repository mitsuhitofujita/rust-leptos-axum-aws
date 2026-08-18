//! The `Action type` field of the create-action form: a compact selector and
//! the searchable modal it opens.
//!
//! Mirrors [`crate::icon_picker::IconField`]'s shape exactly — DR-0013's
//! compact-selector-plus-modal-picker pattern, generalized from choosing an
//! icon to choosing a registered action type. What differs is the source of
//! rows: `IconField` reads a 725-row compile-time catalog and defers building
//! them until the dialog first opens; `TypeField` reads the account's own
//! action types, loaded once by the page above it and small enough to render
//! eagerly with no such deferral.

use std::sync::Arc;

use leptos::html::{Button, Dialog, Input};
use leptos::prelude::*;
use shared::ActionType;

use crate::icons::{ActivityGlyph, Check, ChevronRight, Close};

/// `type_id` is the form's own signal, written exactly once per visit to the
/// picker — when `Use selected type` is activated, mirroring `IconField`'s
/// `icon` (DR-0013). `types` is the account's full list; the caller decides
/// what to do when it is empty; this component assumes at least one entry.
///
/// `Arc`, not `Rc`: Leptos's signals require `Send + Sync` even in this
/// single-threaded CSR build, unlike `IconField`'s `&'static [Icon]`, which
/// gets both for free.
#[component]
pub fn TypeField(type_id: RwSignal<String>, types: Vec<ActionType>) -> impl IntoView {
    let types = Arc::new(types);

    let dialog: NodeRef<Dialog> = NodeRef::new();
    let trigger: NodeRef<Button> = NodeRef::new();
    let search: NodeRef<Input> = NodeRef::new();

    // What the visitor has picked but not yet applied. Separate from
    // `type_id` precisely so that dismissing the dialog can discard it.
    let staged = RwSignal::new(String::new());
    let query = RwSignal::new(String::new());
    let expanded = RwSignal::new(false);

    let selected = {
        let types = types.clone();
        move || {
            let current = type_id.get();
            types
                .iter()
                .find(|candidate| candidate.id == current)
                .cloned()
        }
    };

    let needle = Memo::new(move |_| query.get().trim().to_lowercase());
    let matches = {
        let types = types.clone();
        move |id: &str| {
            types
                .iter()
                .find(|candidate| candidate.id == id)
                .is_some_and(|candidate| candidate.name.to_lowercase().contains(&needle.get()))
        }
    };
    let count = {
        let types = types.clone();
        let matches = matches.clone();
        Memo::new(move |_| {
            types
                .iter()
                .filter(|candidate| matches(&candidate.id))
                .count()
        })
    };

    let open = move |_| {
        staged.set(type_id.get_untracked());
        query.set(String::new());
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

    let apply = {
        let types = types.clone();
        move |_| {
            let choice = staged.get_untracked();
            // Only an id this list actually has, so a stale staged value
            // cannot become a stored one.
            if types.iter().any(|candidate| candidate.id == choice) {
                type_id.set(choice);
            }
            close();
        }
    };

    // Everything that dismisses the dialog ends here — Escape, the close
    // control, and applying a choice — so returning focus is written once.
    let dismissed = move |_| {
        expanded.set(false);
        if let Some(element) = trigger.get_untracked() {
            let _ = element.focus();
        }
    };

    // Disabled when the staged choice is not among the rows on screen, so
    // that applying can never do something the visitor cannot see.
    let cannot_apply = {
        let matches = matches.clone();
        move || !matches(&staged.get())
    };

    let rows = {
        let types = types.clone();
        let matches = matches.clone();
        move || {
            types
                .iter()
                .map(|candidate| {
                    let id = candidate.id.clone();
                    let name = candidate.name.clone();
                    let unit = candidate.unit.clone();
                    let icon = candidate.icon.clone();
                    let matches = matches.clone();
                    let row_id = id.clone();
                    let checked_id = id.clone();
                    let change_id = id.clone();

                    view! {
                        <label class="type-result" hidden=move || !matches(&row_id)>
                            <input
                                type="radio"
                                name="picker-type"
                                value=id
                                prop:checked=move || staged.get() == checked_id
                                on:change=move |_| staged.set(change_id.clone())
                            />
                            <span class="type-result-surface">
                                <span class="result-icon" aria-hidden="true">
                                    <ActivityGlyph icon=icon />
                                </span>
                                <span class="result-copy">
                                    <span class="result-name">{name}</span>
                                </span>
                                <span class="result-unit">{unit}</span>
                                <span class="result-check" aria-hidden="true"><Check /></span>
                            </span>
                        </label>
                    }
                })
                .collect_view()
        }
    };

    view! {
        <div class="field">
            <span class="field-label" id="type-field-label">"Action type"</span>
            <button
                class="type-select"
                type="button"
                node_ref=trigger
                aria-haspopup="dialog"
                aria-controls="type-picker-dialog"
                aria-describedby="type-help"
                aria-expanded=move || expanded.get().to_string()
                aria-labelledby="type-field-label selected-type-name"
                on:click=open
            >
                <span class="type-select-icon" aria-hidden="true">
                    {
                        let selected = selected.clone();
                        move || view! { <ActivityGlyph icon=selected().map(|t| t.icon).unwrap_or_default() /> }
                    }
                </span>
                <span class="type-select-copy">
                    <span class="type-select-name" id="selected-type-name">
                        {
                            let selected = selected.clone();
                            move || selected().map(|t| t.name).unwrap_or_default()
                        }
                    </span>
                </span>
                <span class="type-select-unit">
                    {move || selected().map(|t| t.unit).unwrap_or_default()}
                </span>
                <span class="type-select-chevron" aria-hidden="true"><ChevronRight /></span>
            </button>
            <p class="field-help" id="type-help">
                "Choose which registered type this record belongs to."
            </p>
        </div>

        <dialog
            class="type-dialog"
            id="type-picker-dialog"
            node_ref=dialog
            aria-labelledby="type-picker-title"
            on:close=dismissed
        >
            <div class="type-dialog-header">
                <div>
                    <p class="type-dialog-label">"Action type"</p>
                    <h2 id="type-picker-title">"Choose a type"</h2>
                </div>
                <button
                    class="type-dialog-close"
                    type="button"
                    aria-label="Close type picker"
                    on:click=move |_| close()
                >
                    <Close />
                </button>
            </div>

            <label class="search-field">
                <span class="field-label">"Search types"</span>
                <input
                    class="type-search"
                    type="search"
                    placeholder="e.g. running"
                    autocomplete="off"
                    node_ref=search
                    on:input=move |event| query.set(event_target_value(&event))
                />
            </label>

            <p class="results-meta" aria-live="polite">
                {move || {
                    let found = count.get();
                    if found == 1 { "1 type".to_owned() } else { format!("{found} types") }
                }}
            </p>

            <div class="type-results" role="radiogroup" aria-label="Your action types">
                {rows}
            </div>

            <Show when=move || count.get() == 0>
                <p class="empty-results">"No action types match your search."</p>
            </Show>

            <button
                class="apply-type-button"
                type="button"
                disabled=cannot_apply
                on:click=apply
            >
                "Use selected type"
            </button>
        </dialog>
    }
}
