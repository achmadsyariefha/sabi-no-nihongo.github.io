use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::empty_state::EmptyState;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <EmptyState kanji="迷子" title="Page not found" message="This page doesn't exist.">
            <A href="/" attr:class="inline-flex px-4 py-2 rounded-full bg-zinc-900 dark:bg-white text-white dark:text-dim-900 text-sm font-semibold">
                "Back to Today"
            </A>
        </EmptyState>
    }
}
