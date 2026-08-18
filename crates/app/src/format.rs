//! Display formatting for an action record's numeric value and timestamp.
//!
//! Factored out once a second screen needed it: `dashboard.rs`'s recent list
//! was the first caller, `actions.rs`'s list, create and edit screens are the
//! rest.

use wasm_bindgen::JsValue;

/// `6200` becomes `6,200`; `5.2` stays `5.2`.
///
/// `{}` on an `f64` already prints the shortest form that round-trips, so the
/// only thing left is grouping the integer part.
pub fn value(value: f64) -> String {
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
pub fn timestamp(recorded_at: &str) -> String {
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
