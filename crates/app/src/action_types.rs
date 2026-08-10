//! The action-type screens: the index of what is registered, and creating one.
//!
//! Both are behind [`crate::app::RequireAuth`], so both may assume a settled
//! auth state and a stored token (DR-0011).

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use shared::{ActionType, NewActionType};

use crate::api::{self, ApiError};
use crate::app::{AccountControl, SiteHeader, note_unauthorized};
use crate::icon_picker::IconField;
use crate::icons::{ActivityGlyph, ChevronRight, Plus};

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
            // Editing has no screen yet, so this is where it will be rather
            // than where it is; the router's fallback answers until then.
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
                    <p class="empty-types">
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
