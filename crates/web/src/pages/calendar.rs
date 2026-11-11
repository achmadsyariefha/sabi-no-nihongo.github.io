use leptos::prelude::*;

use crate::components::empty_state::EmptyState;

#[component]
pub fn CalendarPage() -> impl IntoView {
    view! {
        <EmptyState
            kanji="暦"
            title="Your calendar is empty"
            message="Logged days and review activity will show up here as a calendar and heatmaps."
        />
    }
}
