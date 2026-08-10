//! The dashboard: the authenticated overview and repeat-recording surface.

use leptos::prelude::*;
use leptos_router::components::A;
use shared::{ActionRecord, RecentSummary};
use wasm_bindgen::JsValue;

use crate::api::{ApiError, fetch_dashboard};
use crate::app::{AccountControl, SiteHeader, note_unauthorized};
use crate::icons::{ActivityGlyph, Plus};

#[component]
pub fn DashboardPage() -> impl IntoView {
    // `LocalResource` rather than `Resource`: the browser fetch future is not
    // `Send`, and in a CSR build nothing ever runs on the server.
    //
    // The auth state is settled before this screen exists: `RequireAuth` is what
    // renders it, and it renders nothing while the state is still `Loading`. So
    // the token is already stored when this first runs, and the resource has no
    // reason to watch the signal.
    let dashboard = LocalResource::new(fetch_dashboard);

    // A 401 means the token this tab holds is not one the API accepts. Dropping
    // the session is all that happens here; the guard is what notices and sends
    // the visitor home.
    Effect::new(move || {
        if matches!(dashboard.get(), Some(Err(ApiError::Unauthorized))) {
            note_unauthorized();
        }
    });

    view! {
        <SiteHeader><AccountControl /></SiteHeader>

        <section class="page-heading" aria-labelledby="page-title">
            <p class="eyebrow">"Your progress"</p>
            <h1 id="page-title">"Dashboard"</h1>
        </section>

        <Suspense fallback=|| view! { <p class="status">"Loading your actions…"</p> }>
            {move || Suspend::new(async move {
                match dashboard.await {
                    Ok(data) => view! {
                        <SummaryCard summary=data.summary />
                        <RecentActions records=data.recent />
                    }
                    .into_any(),
                    // No arm for `Unauthorized`: the effect above has already
                    // dropped the session, and the guard is sending the visitor
                    // to the home screen that offers a fresh sign-in.
                    Err(error) => view! {
                        <p class="error-message">{error.to_string()}</p>
                    }
                    .into_any(),
                }
            })}
        </Suspense>
    }
}

/// The accent card: the total in text, and one bar per day beside it.
#[component]
fn SummaryCard(summary: RecentSummary) -> impl IntoView {
    // The tallest day sets the scale, so the chart is a shape rather than a
    // measurement — which is all it is allowed to be, being hidden from
    // assistive technology. The floor keeps a quiet day visible as a mark.
    let tallest = summary.daily.iter().copied().max().unwrap_or(0).max(1);
    let last = summary.daily.len().saturating_sub(1);
    let bars = summary
        .daily
        .iter()
        .copied()
        .enumerate()
        .map(|(index, count)| {
            let height = (f64::from(count) / f64::from(tallest) * 100.0).max(8.0);
            let class = if index == last {
                "bar bar-today"
            } else {
                "bar"
            };
            view! { <span class=class style=format!("height: {height:.0}%")></span> }
        })
        .collect_view();

    view! {
        <section class="summary-card" aria-label="Recent 10-day summary">
            <p class="summary-label">"Recent"</p>
            <div class="recent-chart" aria-hidden="true">{bars}</div>
            <p class="summary-number">
                {summary.total}
                <span class="summary-unit">"actions"</span>
            </p>
        </section>
    }
}

/// The ten latest records, newest first. Each row is one link, because it
/// represents one action.
#[component]
fn RecentActions(records: Vec<ActionRecord>) -> impl IntoView {
    let count = records.len();
    let label = if count == 1 {
        "1 record".to_owned()
    } else {
        format!("{count} records")
    };

    let rows = records
        .into_iter()
        .map(|record| {
            // Recording the same activity again is one transition: the row opens
            // action creation with its type already selected. That screen has no
            // defined layout yet, so this is where it will be, not where it is.
            let href = format!("/actions/new?action_type={}", record.action_type.id);
            let value = format_value(record.value);
            let time = format_timestamp(&record.recorded_at);

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
                        // Reinforces the transition the row already states.
                        <span class="repeat-icon" aria-hidden="true"><Plus /></span>
                    </A>
                </li>
            }
        })
        .collect_view();

    view! {
        <section aria-labelledby="recent-title">
            <div class="section-heading">
                <h2 id="recent-title">"Recent actions"</h2>
                <span class="record-count">{label}</span>
            </div>
            <p class="helper">"Last 10 days · Tap an action to record it again."</p>
            <ol class="activity-list">{rows}</ol>
        </section>
    }
}

/// `6200` becomes `6,200`; `5.2` stays `5.2`.
///
/// `{}` on an `f64` already prints the shortest form that round-trips, so the
/// only thing left is grouping the integer part.
fn format_value(value: f64) -> String {
    let text = format!("{value}");
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (text.as_str(), None),
    };

    let (sign, digits) = match whole.strip_prefix('-') {
        Some(digits) => ("-", digits),
        None => ("", whole),
    };

    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }

    match fraction {
        Some(fraction) => format!("{sign}{grouped}.{fraction}"),
        None => format!("{sign}{grouped}"),
    }
}

/// An RFC 3339 instant as `YYYY-MM-DD HH:MM` in the viewer's own time zone.
///
/// The browser is what knows that zone, so the conversion belongs here rather
/// than in the API. An instant it cannot parse is shown as it arrived, which is
/// wrong-looking rather than missing.
fn format_timestamp(recorded_at: &str) -> String {
    let date = js_sys::Date::new(&JsValue::from_str(recorded_at));
    if date.get_time().is_nan() {
        return recorded_at.to_owned();
    }

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date(),
        date.get_hours(),
        date.get_minutes(),
    )
}
