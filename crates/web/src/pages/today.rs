use leptos::prelude::*;
use sabi_core::proverbs::proverb_for;

use crate::components::empty_state::EmptyState;
use crate::services::clock;

#[component]
pub fn TodayPage() -> impl IntoView {
    let today = clock::today();
    let proverb = proverb_for(today);

    view! {
        <div class="space-y-6">
            <div class="flex flex-col sm:flex-row sm:items-end sm:justify-between gap-2">
                <div>
                    <p class="text-xs font-semibold uppercase tracking-wider text-sakura-600 dark:text-sakura-400">"Today"</p>
                    <h1 class="font-jp text-2xl font-bold text-zinc-900 dark:text-white">{today.format_ja()}</h1>
                </div>
                <div class="rounded-2xl border border-zinc-200/80 dark:border-dim-800 bg-paper dark:bg-dim-900 px-4 py-3 sm:text-right">
                    <p class="font-jp-serif font-semibold text-zinc-900 dark:text-white">{proverb.japanese}</p>
                    <p class="text-xs text-zinc-500 dark:text-dim-400">
                        <span class="font-jp">{proverb.reading}</span>" · "{proverb.translation}
                    </p>
                </div>
            </div>
            <EmptyState
                kanji="一日一語"
                title="Log your first word today"
                message="Pick a Japanese word you love and write down its reading and meaning. The entry form arrives in Phase 1."
            />
        </div>
    }
}
