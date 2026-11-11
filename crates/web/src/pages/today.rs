use leptos::prelude::*;
use sabi_core::model::entries_on;
use sabi_core::proverbs::proverb_for;

use crate::components::entry_card::EntryCard;
use crate::components::entry_form::EntryForm;
use crate::components::icon::{Icon, IconName};
use crate::components::stat_cards::StatCards;
use crate::state::use_app_state;

const PAGER_BTN: &str = "p-2 rounded-full border border-zinc-200 dark:border-dim-700 text-zinc-600 dark:text-dim-300 hover:bg-zinc-100 dark:hover:bg-dim-800 disabled:opacity-30 disabled:hover:bg-transparent";

#[component]
pub fn TodayPage() -> impl IntoView {
    let state = use_app_state();
    let today_entries = Memo::new(move |_| {
        let day = state.today.get();
        state.entries.with(|list| entries_on(list, day))
    });
    // Which of today's words is shown; usize::MAX means "the newest".
    let index = RwSignal::new(usize::MAX);
    let adding = RwSignal::new(false);
    let goal_met = move || state.daily_limit().is_met(today_entries.with(|l| l.len()));

    let on_edit = Callback::new(move |id: String| state.editing.set(Some(id)));

    let main = move || {
        // 1. Editing an existing word (from today, or from the Vault).
        if let Some(entry) = state.editing.get().and_then(|id| state.find(&id)) {
            let close = Callback::new(move |_: ()| state.editing.set(None));
            return view! {
                <EntryForm
                    original=entry
                    badge="Editing"
                    title="Edit word"
                    on_saved=Callback::new(move |_| state.editing.set(None))
                    on_cancel=Some(close)
                />
            }
            .into_any();
        }

        let list = today_entries.get();

        // 2. Nothing logged today, or adding another word.
        if list.is_empty() || adding.get() {
            let first = list.is_empty();
            // Untracked: changing the goal must not rebuild (and wipe) the form.
            let met = untrack(goal_met);
            return view! {
                {first
                    .then(|| {
                        view! {
                            <div class="text-center py-2">
                                <p class="font-jp-serif text-4xl sm:text-5xl font-bold text-zinc-900 dark:text-white">"一日一語"</p>
                                <p class="mt-2 text-sm text-zinc-500 dark:text-dim-400">
                                    "Log your first word today: a word you love, its reading, and its meaning."
                                </p>
                            </div>
                        }
                    })}
                <EntryForm
                    badge=if met { "Bonus word" } else { "Today's word" }
                    title="Add a Japanese word"
                    on_saved=Callback::new(move |_| {
                        adding.set(false);
                        index.set(usize::MAX);
                    })
                    on_cancel=(!first).then(|| Callback::new(move |_: ()| adding.set(false)))
                />
            }
            .into_any();
        }

        // 3. Today's words, one at a time.
        let n = list.len();
        let i = index.get().min(n - 1);
        let entry = list[i].clone();
        let at_start = i == 0;
        let at_end = i + 1 >= n;
        let met = goal_met();
        let count = n;
        let goal = state.daily_limit().count().unwrap_or(0);
        view! {
            <div class="space-y-4">
                <EntryCard entry on_edit />
                <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
                    <div class="flex items-center gap-2">
                        <Show when=move || { n > 1 }>
                            <button
                                type="button"
                                class=PAGER_BTN
                                aria-label="Previous word"
                                disabled={at_start}
                                on:click=move |_| index.set(i.saturating_sub(1))
                            >
                                <Icon name=IconName::ChevronLeft />
                            </button>
                            <span class="text-sm tabular-nums text-zinc-500 dark:text-dim-400">{i + 1} " / " {n}</span>
                            <button
                                type="button"
                                class=PAGER_BTN
                                aria-label="Next word"
                                disabled={at_end}
                                on:click=move |_| index.set(i + 1)
                            >
                                <Icon name=IconName::ChevronRight />
                            </button>
                        </Show>
                    </div>
                    <div class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-3">
                        {met
                            .then(|| {
                                view! {
                                    <span class="text-sm text-matcha-700 dark:text-matcha-400">
                                        "Daily goal reached (" {count} "/" {goal} "). Want to record another?"
                                    </span>
                                }
                            })}
                        <button
                            type="button"
                            class="inline-flex items-center justify-center gap-1.5 px-4 py-2 rounded-full text-sm font-semibold bg-sakura-500 text-white hover:bg-sakura-600"
                            on:click=move |_| adding.set(true)
                        >
                            <Icon name=IconName::Plus />
                            {if met { "Add bonus word" } else { "Add another word" }}
                        </button>
                    </div>
                </div>
            </div>
        }
        .into_any()
    };

    view! {
        <div class="space-y-6">
            <div class="flex flex-col sm:flex-row sm:items-end sm:justify-between gap-3">
                <div>
                    <p class="text-xs font-semibold uppercase tracking-wider text-sakura-600 dark:text-sakura-400">"Today"</p>
                    <h1 class="font-jp text-2xl font-bold text-zinc-900 dark:text-white">
                        {move || state.today.get().format_ja()}
                    </h1>
                </div>
                {move || {
                    let p = proverb_for(state.today.get());
                    view! {
                        <div class="rounded-2xl border border-zinc-200/80 dark:border-dim-800 bg-paper dark:bg-dim-900 px-4 py-3 sm:text-right">
                            <p class="font-jp-serif font-semibold text-zinc-900 dark:text-white">{p.japanese}</p>
                            <p class="text-xs text-zinc-500 dark:text-dim-400">
                                <span class="font-jp">{p.reading}</span>
                                " · "
                                {p.translation}
                            </p>
                        </div>
                    }
                }}
            </div>
            <StatCards />
            {main}
        </div>
    }
}
