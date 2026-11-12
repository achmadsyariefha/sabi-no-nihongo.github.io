//! Light/dark mode helpers.

use leptos::prelude::{document, window};

pub fn system_prefers_dark() -> bool {
    window()
        .match_media("(prefers-color-scheme: dark)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

/// The user asked the OS for less animation.
pub fn prefers_reduced_motion() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

pub fn apply_dark_class(dark: bool) {
    if let Some(root) = document().document_element() {
        let _ = root.class_list().toggle_with_force("dark", dark);
    }
}
