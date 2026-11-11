//! Unique ids for new entries.

use leptos::prelude::window;

/// A random UUID from `crypto.randomUUID()`, with a time+random fallback for
/// insecure contexts where that API is missing.
pub fn new_id() -> String {
    window()
        .crypto()
        .ok()
        .map(|c| c.random_uuid())
        .unwrap_or_else(|| {
            let rand = (js_sys::Math::random() * 1e12) as u64;
            format!("entry-{}-{rand:x}", js_sys::Date::now() as u64)
        })
}
