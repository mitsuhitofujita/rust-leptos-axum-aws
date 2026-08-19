//! The action-record screens: the history of what has been recorded,
//! recording one, and editing or deleting one.
//!
//! All three are behind [`crate::app::RequireAuth`], so all three may assume a
//! settled auth state and a stored token (DR-0011). The shape mirrors
//! [`crate::action_types`] throughout — `LocalResource` + `Suspense` for
//! reads, `spawn_local` with local `saving`/`deleting` signals for writes, a
//! native `<dialog>` for delete confirmation — except that a record's type
//! and its copied display attributes are fixed once created (DR-0016), so
//! editing sends only a value.

use leptos::ev::SubmitEvent;
use leptos::html::Dialog;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use shared::{ActionRecord, ActionType, NewActionRecord, UpdateActionRecord};

use crate::api::{self, ApiError};
use crate::app::{AccountControl, SiteHeader, note_unauthorized};
use crate::format;
use crate::icons::{ActivityGlyph, Checkmark, ChevronRight, Plus, Trash};
use crate::type_picker::TypeField;

/// Where the index lives, which is where every exit from the other two
/// screens leads.
const INDEX: &str = "/actions";

/// The authenticated history: every action record, newest first.
#[component]
pub fn ActionsPage() -> impl IntoView {
    let records = LocalResource::new(api::fetch_action_records);

    // As on the action-types index and the dashboard: a 401 drops the session
    // here and the guard is what moves the visitor.
    Effect::new(move || {
        if matches!(records.get(), Some(Err(ApiError::Unauthorized))) {
            note_unauthorized();
        }
    });

    view! {
        <SiteHeader><AccountControl /></SiteHeader>

        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Your history"</p>
            <h1 id="page-title">"Actions"</h1>
            <p class="lead">"Every action you've recorded, newest first."</p>
        </section>

        <A href="/actions/new" attr:class="add-button">
            <span>"Add action"</span>
            <span class="add-icon" aria-hidden="true"><Plus /></span>
        </A>

        <Suspense fallback=|| view! { <p class="status">"Loading your actions…"</p> }>
            {move || Suspend::new(async move {
                match records.await {
                    Ok(records) => view! { <RecordList records=records /> }.into_any(),
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

/// The recorded actions, or the state that says there are none yet. Reuses
/// the dashboard's `.activity-*` row classes — the shape is identical — with
/// the action-types index's `.edit-icon` chevron in place of the dashboard's
/// `.repeat-icon` plus, since a row here opens editing, not creation.
#[component]
fn RecordList(records: Vec<ActionRecord>) -> impl IntoView {
    let count = records.len();
    let label = if count == 1 {
        "1 record".to_owned()
    } else {
        format!("{count} records")
    };

    let rows = records
        .into_iter()
        .map(|record| {
            let href = format!("{INDEX}/{}", record.id);
            let value = format::value(record.value);
            let time = format::timestamp(&record.recorded_at);

            view! {
                <li class="activity-item">
                    <A href=href attr:class="activity-link">
                        <span class="activity-icon" aria-hidden="true">
                            <ActivityGlyph icon=record.action_type.icon />
                        </span>
                        <span class="activity-copy">
                            <span class="activity-name">{record.action_type.name}</span>
                            <span class="activity-time">{time}</span>
                        </span>
                        <span class="activity-value">
                            {value}" "<span>{record.action_type.unit}</span>
                        </span>
                        <span class="edit-icon" aria-hidden="true"><ChevronRight /></span>
                    </A>
                </li>
            }
        })
        .collect_view();

    view! {
        <section aria-labelledby="records-title">
            <div class="section-heading">
                <h2 id="records-title">"Your actions"</h2>
                <span class="record-count">{label}</span>
            </div>
            <p class="helper">"Tap a record to edit its value or delete it."</p>
            {if count == 0 {
                // No HTML reference defines this state; every account can
                // reach it before its first record, the same way the
                // action-types index can before its first type.
                view! {
                    <p class="empty-state">
                        "Nothing recorded yet. Add an action and it will appear here, newest first."
                    </p>
                }
                .into_any()
            } else {
                view! { <ol class="activity-list">{rows}</ol> }.into_any()
            }}
        </section>
    }
}

/// Recording one action.
#[component]
pub fn NewActionPage() -> impl IntoView {
    // The dashboard's repeat link opens this screen with the row's type
    // already chosen — page-layouts.md requires preserving it.
    let query = use_query_map();
    let preselected = move || query.with(|query| query.get("action_type"));

    let types = LocalResource::new(api::fetch_action_types);

    Effect::new(move || {
        if matches!(types.get(), Some(Err(ApiError::Unauthorized))) {
            note_unauthorized();
        }
    });

    view! {
        <SiteHeader><AccountControl /></SiteHeader>

        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Actions"</p>
            <h1 id="page-title">"Record a"<br /><em>"new action."</em></h1>
        </section>

        <Suspense fallback=|| view! { <p class="status">"Loading your action types…"</p> }>
            {move || Suspend::new(async move {
                match types.await {
                    Ok(types) if types.is_empty() => view! {
                        <p class="empty-state">
                            "You need at least one action type before you can record an action. "
                            <A href="/action-types/new" attr:class="text-link">
                                "Add an action type"
                            </A>
                            "."
                        </p>
                    }
                    .into_any(),
                    Ok(types) => view! {
                        <NewActionForm types=types preselected=preselected() />
                    }
                    .into_any(),
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

/// The create form, once at least one action type has loaded. A separate
/// component from [`NewActionPage`] rather than the body of its `Suspense`,
/// so `types` seeds the form's signals exactly once — the same division
/// [`EditActionForm`] and the action-type screens already draw.
#[component]
fn NewActionForm(types: Vec<ActionType>, preselected: Option<String>) -> impl IntoView {
    let default_type = preselected
        .filter(|id| types.iter().any(|candidate| &candidate.id == id))
        .unwrap_or_else(|| types[0].id.clone());

    let type_id = RwSignal::new(default_type);
    let value_text = RwSignal::new(String::new());

    let types_for_unit = types.clone();
    let selected_unit = move || {
        types_for_unit
            .iter()
            .find(|candidate| candidate.id == type_id.get())
            .map(|candidate| candidate.unit.clone())
            .unwrap_or_default()
    };

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

        let Ok(value) = value_text.get_untracked().trim().parse::<f64>() else {
            error.set(Some("A numeric value is required.".to_owned()));
            return;
        };

        let new = NewActionRecord {
            type_id: type_id.get_untracked(),
            value,
        };

        error.set(None);
        saving.set(true);

        let navigate = navigate.clone();
        spawn_local(async move {
            match api::create_action_record(&new).await {
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
    };

    view! {
        <form class="create-form" on:submit=submit novalidate=false>
            <div class="form-card">
                <TypeField type_id=type_id types=types />

                <div class="field">
                    <label class="field-label" for="value">"Value"</label>
                    <div class="value-input-wrap">
                        <input
                            class="text-input value-input"
                            id="value"
                            type="text"
                            inputmode="decimal"
                            placeholder="e.g. 5.2"
                            autocomplete="off"
                            aria-describedby="value-help"
                            required
                            prop:value=value_text
                            on:input=move |event| value_text.set(event_target_value(&event))
                        />
                        <span class="value-unit" aria-hidden="true">{selected_unit}</span>
                    </div>
                    <p class="field-help" id="value-help">
                        "Recorded in the unit the action type uses."
                    </p>
                </div>
            </div>

            <div class="form-actions">
                <Show when=move || error.get().is_some()>
                    <p class="error-message">{move || error.get().unwrap_or_default()}</p>
                </Show>
                <button class="primary-button" type="submit" disabled=move || saving.get()>
                    <span aria-hidden="true"><Plus /></span>
                    <span>
                        {move || if saving.get() { "Recording action…" } else { "Record action" }}
                    </span>
                </button>
                <A href=INDEX attr:class="cancel-link">"Cancel"</A>
            </div>
        </form>
    }
}

/// Editing a recorded action's value, and deleting it. The route names `id`;
/// [`EditActionForm`] does the rest once the record behind it has loaded.
#[component]
pub fn EditActionPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.with(|params| params.get("id").unwrap_or_default());

    let record = LocalResource::new(move || {
        let id = id();
        async move { api::fetch_action_record(&id).await }
    });

    // As on the other authenticated screens: a 401 drops the session here and
    // the guard is what moves the visitor.
    Effect::new(move || {
        if matches!(record.get(), Some(Err(ApiError::Unauthorized))) {
            note_unauthorized();
        }
    });

    view! {
        <SiteHeader><AccountControl /></SiteHeader>

        <Suspense fallback=|| view! { <p class="status">"Loading this action…"</p> }>
            {move || Suspend::new(async move {
                match record.await {
                    Ok(loaded) => view! { <EditActionForm id=id() initial=loaded /> }.into_any(),
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

/// The heading, the save form and the delete flow, once `initial` has loaded.
#[component]
fn EditActionForm(id: String, initial: ActionRecord) -> impl IntoView {
    let value_text = RwSignal::new(initial.value.to_string());

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

            let Ok(value) = value_text.get_untracked().trim().parse::<f64>() else {
                error.set(Some("A numeric value is required.".to_owned()));
                return;
            };

            error.set(None);
            saving.set(true);

            let id = id.clone();
            let navigate = navigate.clone();
            spawn_local(async move {
                match api::update_action_record(&id, &UpdateActionRecord { value }).await {
                    Ok(_) => navigate(INDEX, NavigateOptions::default()),
                    // Nothing to report on this screen: the session is gone
                    // and the guard is already sending the visitor to a
                    // fresh sign-in.
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
            match api::delete_action_record(&id).await {
                Ok(()) => navigate(INDEX, NavigateOptions::default()),
                Err(ApiError::Unauthorized) => note_unauthorized(),
                Err(failure) => {
                    delete_error.set(Some(failure.to_string()));
                    deleting.set(false);
                }
            }
        });
    };

    let time = format::timestamp(&initial.recorded_at);
    let dialog_time = time.clone();
    let dialog_value = format::value(initial.value);

    view! {
        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Actions"</p>
            <h1 id="page-title">"Edit"<br /><em>"action."</em></h1>
        </section>

        <form class="create-form" on:submit=save novalidate=false>
            <div class="form-card">
                <div class="field">
                    <span class="field-label" id="type-readonly-label">"Action type"</span>
                    <div class="type-readonly" role="group" aria-labelledby="type-readonly-label">
                        <span class="type-readonly-icon" aria-hidden="true">
                            <ActivityGlyph icon=initial.action_type.icon.clone() />
                        </span>
                        <span class="type-readonly-copy">
                            <span class="type-readonly-name">{initial.action_type.name.clone()}</span>
                        </span>
                        <span class="type-readonly-unit">{initial.action_type.unit.clone()}</span>
                    </div>
                    <p class="field-help" id="type-readonly-help">
                        "The type is fixed once a record is created (DR-0016)."
                    </p>
                </div>

                <div class="field">
                    <span class="field-label" id="recorded-at-label">"Recorded"</span>
                    <p class="readonly-value" aria-labelledby="recorded-at-label">{time}</p>
                </div>

                <div class="field">
                    <label class="field-label" for="value">"Value"</label>
                    <div class="value-input-wrap">
                        <input
                            class="text-input value-input"
                            id="value"
                            type="text"
                            inputmode="decimal"
                            autocomplete="off"
                            aria-describedby="value-help"
                            required
                            prop:value=value_text
                            on:input=move |event| value_text.set(event_target_value(&event))
                        />
                        <span class="value-unit" aria-hidden="true">{initial.action_type.unit.clone()}</span>
                    </div>
                    <p class="field-help" id="value-help">
                        "Correct the recorded number if it was entered wrong."
                    </p>
                </div>
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
            <h2 id="delete-title">"Delete this action"</h2>
            <p class="danger-copy">
                "Remove this record if it should not have been recorded."
            </p>
            <Show when=move || delete_error.get().is_some()>
                <p class="error-message">{move || delete_error.get().unwrap_or_default()}</p>
            </Show>
            <button class="delete-button" type="button" on:click=open_confirm>
                <Trash />
                <span>"Delete action"</span>
            </button>
        </section>

        // A native dialog, exactly as the action-type edit screen and
        // `IconField`/`TypeField` use one: `showModal` keeps focus inside and
        // closes on Escape, with nothing written here to reproduce either.
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

            <h2 id="confirm-title">"Delete this record?"</h2>
            <p class="dialog-copy" id="confirm-description">
                "This action record will be permanently removed. "
                "This cannot be undone."
            </p>

            <div class="activity-summary" aria-label="Action to delete">
                <span class="activity-icon" aria-hidden="true">
                    <ActivityGlyph icon=initial.action_type.icon.clone() />
                </span>
                <span class="activity-copy">
                    <span class="activity-name">{initial.action_type.name.clone()}</span>
                    <span class="activity-time">{dialog_time}</span>
                </span>
                <span class="activity-value">
                    {dialog_value}" "<span>{initial.action_type.unit.clone()}</span>
                </span>
            </div>

            <div class="dialog-actions">
                // First in document order and where the dialog sends initial
                // focus, so the default response to opening it never deletes
                // anything.
                <button class="dialog-button keep-button" type="button" autofocus on:click=keep>
                    "Keep action"
                </button>
                <button
                    class="dialog-button delete-button"
                    type="button"
                    disabled=move || deleting.get()
                    on:click=confirm_delete
                >
                    <Trash />
                    <span>
                        {move || if deleting.get() { "Deleting…" } else { "Delete action" }}
                    </span>
                </button>
            </div>
        </dialog>
    }
}
