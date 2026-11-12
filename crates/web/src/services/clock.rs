//! The browser's clock. The only place the app reads the current time.

use sabi_core::DateKey;

/// Today's date in the browser's local timezone (day starts at local midnight).
pub fn today() -> DateKey {
    let now = js_sys::Date::new_0();
    DateKey::new(now.get_full_year() as i32, now.get_month() as u8 + 1, now.get_date() as u8)
        .expect("browser returned a valid date")
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    js_sys::Date::now() as i64
}

/// Local midnight at the start of `day`, in milliseconds since the Unix epoch.
pub fn start_of_day_ms(day: DateKey) -> i64 {
    js_sys::Date::new_with_year_month_day(day.year() as u32, i32::from(day.month()) - 1, i32::from(day.day())).get_time()
        as i64
}

/// The local date of a timestamp.
pub fn date_of(ms: i64) -> DateKey {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms as f64));
    DateKey::new(d.get_full_year() as i32, d.get_month() as u8 + 1, d.get_date() as u8)
        .expect("browser returned a valid date")
}
