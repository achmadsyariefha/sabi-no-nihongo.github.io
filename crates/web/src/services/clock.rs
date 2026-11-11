//! The browser's local date. The only place the app reads the clock.

use sabi_core::DateKey;

/// Today's date in the browser's local timezone (day starts at local midnight).
pub fn today() -> DateKey {
    let now = js_sys::Date::new_0();
    DateKey::new(now.get_full_year() as i32, now.get_month() as u8 + 1, now.get_date() as u8)
        .expect("browser returned a valid date")
}
