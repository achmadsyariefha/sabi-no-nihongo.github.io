use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::empty_state::EmptyState;
use crate::services::base::app_href;

#[component]
pub fn VaultPage() -> impl IntoView {
    view! {
        <EmptyState kanji="語彙" title="No words yet" message="Every word you log will be collected here, searchable by text and tag.">
            <A href=app_href("/") attr:class="inline-flex px-4 py-2 rounded-full bg-zinc-900 dark:bg-white text-white dark:text-dim-900 text-sm font-semibold">
                "Go to Today"
            </A>
        </EmptyState>
    }
}
