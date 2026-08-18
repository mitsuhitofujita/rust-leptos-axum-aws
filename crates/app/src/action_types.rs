//! The action-type screens: the index of what is registered, creating one,
//! and editing or deleting one.
//!
//! All three are behind [`crate::app::RequireAuth`], so all three may assume a
//! settled auth state and a stored token (DR-0011).

use leptos::ev::SubmitEvent;
use leptos::html::Dialog;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use shared::{ActionType, NewActionType};

use crate::api::{self, ApiError};
use crate::app::{AccountControl, SiteHeader, note_unauthorized};
use crate::icon_picker::IconField;
use crate::icons::{ActivityGlyph, Checkmark, ChevronRight, Plus, Trash};

/// Where the index lives, which is where both of the form's exits lead.
const INDEX: &str = "/action-types";

/// What the icon field starts on before anything has been chosen.
///
/// A form that opened on no icon at all would have to hold an empty state for a
/// required field whose control cannot show one, so it opens on a member of the
/// catalog instead. This is the icon the design reference shows.
const DEFAULT_ICON: &str = "person-standing";

/// The authenticated management index.
#[component]
pub fn ActionTypesPage() -> impl IntoView {
    let action_types = LocalResource::new(api::fetch_action_types);

    // As on the dashboard: a 401 drops the session here and the guard is what
    // moves the visitor.
    Effect::new(move || {
        if matches!(action_types.get(), Some(Err(ApiError::Unauthorized))) {
            note_unauthorized();
        }
    });

    view! {
        <SiteHeader><AccountControl /></SiteHeader>

        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Your setup"</p>
            <h1 id="page-title">"Action types"</h1>
            <p class="lead">
                "Keep the actions you record and their numeric units in one place."
            </p>
        </section>

        <A href="/action-types/new" attr:class="add-button">
            <span>"Add action type"</span>
            <span class="add-icon" aria-hidden="true"><Plus /></span>
        </A>

        <Suspense fallback=|| view! { <p class="status">"Loading your action types…"</p> }>
            {move || Suspend::new(async move {
                match action_types.await {
                    Ok(types) => view! { <TypeList types=types /> }.into_any(),
                    // No arm for `Unauthorized`: the effect above has already
                    // dropped the session and the guard is on its way out.
                    Err(error) => view! {
                        <p class="error-message">{error.to_string()}</p>
                    }
                    .into_any(),
                }
            })}
        </Suspense>
    }
}

/// The registered types, or the state that says there are none yet.
#[component]
fn TypeList(types: Vec<ActionType>) -> impl IntoView {
    let count = types.len();
    let label = if count == 1 {
        "1 type".to_owned()
    } else {
        format!("{count} types")
    };

    let rows = types
        .into_iter()
        .map(|action_type| {
            let href = format!("{INDEX}/{}", action_type.id);

            view! {
                <li class="type-item">
                    <A href=href attr:class="type-link">
                        // Supplemental: the name beside it is the row's content.
                        <span class="type-icon" aria-hidden="true">
                            <ActivityGlyph icon=action_type.icon />
                        </span>
                        <span class="type-copy">
                            <span class="type-name">{action_type.name}</span>
                        </span>
                        <span class="unit-value">{action_type.unit}</span>
                        <span class="edit-icon" aria-hidden="true"><ChevronRight /></span>
                    </A>
                </li>
            }
        })
        .collect_view();

    view! {
        <section aria-labelledby="types-title">
            <div class="section-heading">
                <h2 id="types-title">"Your types"</h2>
                <span class="type-count">{label}</span>
            </div>
            <p class="helper">"Choose a type to edit its name, unit, or icon."</p>
            {if count == 0 {
                // Page Layouts defines the populated state only, and every
                // account begins in this one. The control above is already the
                // way out of it, so this says what is missing and nothing more.
                view! {
                    <p class="empty-state">
                        "Nothing registered yet. Add an action type and it will wait here, "
                        "ready to record."
                    </p>
                }
                .into_any()
            } else {
                view! { <ul class="type-list">{rows}</ul> }.into_any()
            }}
        </section>
    }
}

/// Registering one action type.
#[component]
pub fn NewActionTypePage() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let unit = RwSignal::new(String::new());
    let icon = RwSignal::new(DEFAULT_ICON.to_owned());

    let error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);

    let navigate = use_navigate();
    let submit = move |event: SubmitEvent| {
        // The browser would navigate and reload the whole application
        // otherwise; this is a single-page form.
        event.prevent_default();
        if saving.get_untracked() {
            return;
        }

        let new = NewActionType {
            name: name.get_untracked().trim().to_owned(),
            unit: unit.get_untracked().trim().to_owned(),
            icon: icon.get_untracked(),
        };

        // `required` on the inputs means the browser has already refused an
        // empty field. This catches the one it does not: whitespace. Neither is
        // the check that matters — the service validates what it stores, and
        // this only saves a round trip.
        if new.name.is_empty() || new.unit.is_empty() {
            error.set(Some(
                "An action name and a numeric unit are both required.".to_owned(),
            ));
            return;
        }

        error.set(None);
        saving.set(true);

        let navigate = navigate.clone();
        spawn_local(async move {
            match api::create_action_type(&new).await {
                Ok(_) => navigate(INDEX, NavigateOptions::default()),
                // Nothing to report on this screen: the session is gone and the
                // guard is already sending the visitor to a fresh sign-in.
                Err(ApiError::Unauthorized) => note_unauthorized(),
                Err(failure) => {
                    error.set(Some(failure.to_string()));
                    saving.set(false);
                }
            }
        });
    };

    view! {
        <SiteHeader><AccountControl /></SiteHeader>

        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Action types"</p>
            <h1 id="page-title">"Create a"<br /><em>"new type."</em></h1>
        </section>

        <form class="create-form" on:submit=submit novalidate=false>
            <div class="form-card">
                <div class="field">
                    <label class="field-label" for="action-name">"Action name"</label>
                    <input
                        class="text-input"
                        id="action-name"
                        type="text"
                        placeholder="e.g. Running"
                        autocomplete="off"
                        aria-describedby="action-name-help"
                        required
                        prop:value=name
                        on:input=move |event| name.set(event_target_value(&event))
                    />
                    <p class="field-help" id="action-name-help">
                        "Use the name you want to see in every record."
                    </p>
                </div>

                <div class="field">
                    <label class="field-label" for="unit">"Numeric unit"</label>
                    <input
                        class="text-input"
                        id="unit"
                        type="text"
                        placeholder="e.g. km"
                        autocomplete="off"
                        aria-describedby="unit-help"
                        required
                        prop:value=unit
                        on:input=move |event| unit.set(event_target_value(&event))
                    />
                    <p class="field-help" id="unit-help">
                        "This appears beside the value, such as 5.2 km."
                    </p>
                </div>

                <IconField icon=icon />
            </div>

            <div class="form-actions">
                <Show when=move || error.get().is_some()>
                    <p class="error-message">{move || error.get().unwrap_or_default()}</p>
                </Show>
                <button class="primary-button" type="submit" disabled=move || saving.get()>
                    <span aria-hidden="true"><Plus /></span>
                    <span>
                        {move || {
                            if saving.get() { "Creating action type…" } else { "Create action type" }
                        }}
                    </span>
                </button>
                <A href=INDEX attr:class="cancel-link">"Cancel"</A>
            </div>
        </form>
    }
}

/// Editing a registered action type, and deleting it. The route names `id`;
/// [`EditForm`] does the rest once the type behind it has loaded.
#[component]
pub fn EditActionTypePage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.with(|params| params.get("id").unwrap_or_default());

    let action_type = LocalResource::new(move || {
        let id = id();
        async move { api::fetch_action_type(&id).await }
    });

    // As on the index and the dashboard: a 401 drops the session here and the
    // guard is what moves the visitor.
    Effect::new(move || {
        if matches!(action_type.get(), Some(Err(ApiError::Unauthorized))) {
            note_unauthorized();
        }
    });

    view! {
        <SiteHeader><AccountControl /></SiteHeader>

        <Suspense fallback=|| view! { <p class="status">"Loading this action type…"</p> }>
            {move || Suspend::new(async move {
                match action_type.await {
                    Ok(loaded) => view! { <EditForm id=id() initial=loaded /> }.into_any(),
                    // No arm for `Unauthorized`: the effect above has already
                    // dropped the session and the guard is on its way out.
                    Err(error) => view! {
                        <p class="error-message">{error.to_string()}</p>
                    }
                    .into_any(),
                }
            })}
        </Suspense>
    }
}

/// The heading, the save form, and the delete flow, once `initial` has loaded.
///
/// A separate component from [`EditActionTypePage`] rather than the body of
/// its `Suspense`, so `initial`'s fields seed the form's signals exactly once,
/// the same way [`NewActionTypePage`]'s start empty exactly once.
#[component]
fn EditForm(id: String, initial: ActionType) -> impl IntoView {
    let name = RwSignal::new(initial.name.clone());
    let unit = RwSignal::new(initial.unit.clone());
    let icon = RwSignal::new(initial.icon.clone());

    let error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);

    let delete_error = RwSignal::new(None::<String>);
    let deleting = RwSignal::new(false);

    let confirm_dialog: NodeRef<Dialog> = NodeRef::new();

    let navigate = use_navigate();

    let save = {
        let id = id.clone();
        let navigate = navigate.clone();
        move |event: SubmitEvent| {
            // The browser would navigate and reload the whole application
            // otherwise; this is a single-page form.
            event.prevent_default();
            if saving.get_untracked() {
                return;
            }

            let new = NewActionType {
                name: name.get_untracked().trim().to_owned(),
                unit: unit.get_untracked().trim().to_owned(),
                icon: icon.get_untracked(),
            };

            // Mirrors creation: the browser's `required` has already refused an
            // empty field, this catches whitespace, and the service validates
            // what it stores regardless.
            if new.name.is_empty() || new.unit.is_empty() {
                error.set(Some(
                    "An action name and a numeric unit are both required.".to_owned(),
                ));
                return;
            }

            error.set(None);
            saving.set(true);

            let id = id.clone();
            let navigate = navigate.clone();
            spawn_local(async move {
                match api::update_action_type(&id, &new).await {
                    Ok(_) => navigate(INDEX, NavigateOptions::default()),
                    // Nothing to report on this screen: the session is gone and
                    // the guard is already sending the visitor to a fresh
                    // sign-in.
                    Err(ApiError::Unauthorized) => note_unauthorized(),
                    Err(failure) => {
                        error.set(Some(failure.to_string()));
                        saving.set(false);
                    }
                }
            });
        }
    };

    let open_confirm = move |_| {
        if let Some(element) = confirm_dialog.get_untracked() {
            let _ = element.show_modal();
        }
    };

    let keep = move |_| {
        if let Some(element) = confirm_dialog.get_untracked() {
            element.close();
        }
    };

    let confirm_delete = move |_| {
        if deleting.get_untracked() {
            return;
        }
        deleting.set(true);
        delete_error.set(None);

        let id = id.clone();
        let navigate = navigate.clone();
        spawn_local(async move {
            match api::delete_action_type(&id).await {
                Ok(()) => navigate(INDEX, NavigateOptions::default()),
                Err(ApiError::Unauthorized) => note_unauthorized(),
                Err(failure) => {
                    delete_error.set(Some(failure.to_string()));
                    deleting.set(false);
                }
            }
        });
    };

    view! {
        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Action types"</p>
            <h1 id="page-title">"Edit action"<br /><em>"type."</em></h1>
        </section>

        <form class="create-form" on:submit=save novalidate=false>
            <div class="form-card">
                <div class="field">
                    <label class="field-label" for="action-name">"Action name"</label>
                    <input
                        class="text-input"
                        id="action-name"
                        type="text"
                        autocomplete="off"
                        aria-describedby="action-name-help"
                        required
                        prop:value=name
                        on:input=move |event| name.set(event_target_value(&event))
                    />
                    <p class="field-help" id="action-name-help">
                        "This name appears in every record using this type."
                    </p>
                </div>

                <div class="field">
                    <label class="field-label" for="unit">"Numeric unit"</label>
                    <input
                        class="text-input"
                        id="unit"
                        type="text"
                        autocomplete="off"
                        aria-describedby="unit-help"
                        required
                        prop:value=unit
                        on:input=move |event| unit.set(event_target_value(&event))
                    />
                    <p class="field-help" id="unit-help">
                        "This appears beside the value, such as 5.2 km."
                    </p>
                </div>

                <IconField icon=icon />
            </div>

            <div class="form-actions">
                <Show when=move || error.get().is_some()>
                    <p class="error-message">{move || error.get().unwrap_or_default()}</p>
                </Show>
                <button class="primary-button" type="submit" disabled=move || saving.get()>
                    <span aria-hidden="true"><Checkmark /></span>
                    <span>
                        {move || if saving.get() { "Saving changes…" } else { "Save changes" }}
                    </span>
                </button>
                <A href=INDEX attr:class="cancel-link">"Cancel"</A>
            </div>
        </form>

        <section class="danger-zone" aria-labelledby="delete-title">
            <h2 id="delete-title">"Delete this action type"</h2>
            <p class="danger-copy">
                "Remove this type when you no longer want it available for new records."
            </p>
            <Show when=move || delete_error.get().is_some()>
                <p class="error-message">{move || delete_error.get().unwrap_or_default()}</p>
            </Show>
            <button class="delete-button" type="button" on:click=open_confirm>
                <Trash />
                <span>"Delete action type"</span>
            </button>
        </section>

        // A native dialog, exactly as `IconField` uses one: `showModal` keeps
        // focus inside and closes on Escape, with nothing written here to
        // reproduce either.
        <dialog
            class="confirm-dialog"
            node_ref=confirm_dialog
            role="alertdialog"
            aria-labelledby="confirm-title"
            aria-describedby="confirm-description"
        >
            <p class="dialog-label">
                <span class="dialog-icon" aria-hidden="true"><Trash /></span>
                <span>"Confirm deletion"</span>
            </p>

            <h2 id="confirm-title">{format!("Delete {}?", initial.name)}</h2>
            <p class="dialog-copy" id="confirm-description">
                "This action type will no longer be available for new records. "
                "This cannot be undone."
            </p>

            <div class="type-summary" aria-label="Action type to delete">
                <span class="type-copy">
                    <span class="type-name">{initial.name.clone()}</span>
                    <span class="type-kind">"Action type"</span>
                </span>
                <span class="unit-value">{initial.unit.clone()}</span>
            </div>

            <div class="dialog-actions">
                // First in document order and where the dialog sends initial
                // focus, so the default response to opening it never deletes
                // anything.
                <button class="dialog-button keep-button" type="button" autofocus on:click=keep>
                    "Keep action type"
                </button>
                <button
                    class="dialog-button delete-button"
                    type="button"
                    disabled=move || deleting.get()
                    on:click=confirm_delete
                >
                    <Trash />
                    <span>
                        {move || if deleting.get() { "Deleting…" } else { "Delete action type" }}
                    </span>
                </button>
            </div>
        </dialog>
    }
}
