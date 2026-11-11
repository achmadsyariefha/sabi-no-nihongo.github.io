use leptos::prelude::*;

use crate::components::empty_state::EmptyState;

#[component]
pub fn ReviewPage() -> impl IntoView {
    view! {
        <EmptyState
            kanji="復習"
            title="No cards to review"
            message="Once you have logged some words, review them here as flashcards. Progress resets at local midnight."
        />
    }
}
