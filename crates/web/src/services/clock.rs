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
