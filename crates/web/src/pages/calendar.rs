use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use sabi_core::activity::words_per_day;
use sabi_core::calendar::{Month, summarize};
use sabi_core::model::entries_on;

use crate::components::calendar_grid::CalendarGrid;
use crate::components::entry_card::EntryCard;
use crate::components::heatmap::{Heatmap, Palette};
use crate::components::icon::{Icon, IconName};
use crate::state::use_app_state;

const CARD: &str = "bg-white dark:bg-dim-900 rounded-2xl border border-zinc-200/80 dark:border-dim-800 p-5";
const NAV_BTN: &str = "p-1 rounded-lg text-zinc-600 dark:text-dim-300 hover:bg-zinc-100 dark:hover:bg-dim-800 disabled:opacity-30";

#[component]
pub fn CalendarPage() -> impl IntoView {
    let state = use_app_state();
    let navigate = use_navigate();

    let today = state.today.get_untracked();
    let month = RwSignal::new(Month::of(today));
    let selected = RwSignal::new(today);
    // Which of the selected day's words is shown.
    let index = RwSignal::new(0usize);
    Effect::new(move |_| {
        selected.track();
        index.set(0);
    });

    let days = Memo::new(move |_| state.entries.with(|list| summarize(list)));
    let word_counts = Memo::new(move |_| state.entries.with(|list| words_per_day(list)));
    let day_entries = Memo::new(move |_| {
        let day = selected.get();
        state.entries.with(|list| entries_on(list, day))
    });
    let logged_this_month = move || {
        let m = month.get();
        days.with(|d| d.keys().filter(|k| m.contains(**k)).count())
    };

    let go_today = move |_| {
        let t = state.today.get_untracked();
        month.set(Month::of(t));
        selected.set(t);
    };

    // navigate() adds the router base itself, so pass the bare path.
    let nav = navigate.clone();
    let on_edit = Callback::new(move |id: String| {
        state.editing.set(Some(id));
        nav("/", Default::default());
    });
    let log_today = move |_| {
        state.editing.set(None);
        navigate("/", Default::default());
    };

    let panel = move || {
        let list = day_entries.get();
        let is_today = selected.get() == state.today.get();
        if list.is_empty() {
            let action = if is_today {
                view! {
                    <button
                        type="button"
                        class="inline-flex items-center gap-1.5 px-4 py-2 rounded-full bg-sakura-500 hover:bg-sakura-600 text-white text-sm font-semibold"
                        on:click=log_today.clone()
                    >
                        <Icon name=IconName::Plus />
                        "Log today's word"
                    </button>
                }
                .into_any()
            } else {
                view! { <p class="text-xs text-zinc-400 italic">"You can only log new words for today."</p> }.into_any()
            };
            return view! {
                <div class="py-10 text-center space-y-3">
                    <p class="text-sm text-zinc-500 dark:text-dim-400">"No words logged on this day."</p>
                    {action}
                </div>
            }
            .into_any();
        }
        let n = list.len();
        let i = index.get().min(n - 1);
        let entry = list[i].clone();
        let at_start = i == 0;
        let at_end = i + 1 >= n;
        view! {
            <div class="space-y-3">
                {(n > 1)
                    .then(|| {
                        view! {
                            <div class="flex items-center justify-between">
                                <button
                                    type="button"
                                    class=NAV_BTN
                                    aria-label="Previous word"
                                    disabled={at_start}
                                    on:click=move |_| index.set(i.saturating_sub(1))
                                >
                                    <Icon name=IconName::ChevronLeft />
                                </button>
                                <span class="text-xs font-bold tabular-nums text-zinc-500 dark:text-dim-400">
                                    {i + 1} " / " {n}
                                </span>
                                <button
                                    type="button"
                                    class=NAV_BTN
                                    aria-label="Next word"
                                    disabled={at_end}
                                    on:click=move |_| index.set(i + 1)
                                >
                                    <Icon name=IconName::ChevronRight />
                                </button>
                            </div>
                        }
                    })}
                <EntryCard entry compact=true on_edit />
            </div>
        }
        .into_any()
    };

    view! {
        <div class="space-y-6">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div>
                    <h1 class="text-2xl font-bold font-jp-serif text-zinc-900 dark:text-white flex items-center gap-2">
                        <Icon name=IconName::Calendar class="w-6 h-6 text-sakura-500" />
                        "Calendar"
                    </h1>
                    <p class="text-xs text-zinc-500 dark:text-dim-400 mt-0.5">
                        "Every day you logged a word, month by month."
                    </p>
                </div>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        class="px-3 py-1.5 rounded-xl bg-white dark:bg-dim-900 border border-zinc-200 dark:border-dim-800 text-xs font-semibold text-zinc-700 dark:text-dim-300 hover:bg-zinc-50 dark:hover:bg-dim-800"
                        on:click=go_today
                    >
                        "This month"
                    </button>
                    <div class="flex items-center bg-white dark:bg-dim-900 border border-zinc-200 dark:border-dim-800 rounded-xl p-1">
                        <button
                            type="button"
                            class=NAV_BTN
                            aria-label="Previous month"
                            on:click=move |_| month.update(|m| *m = m.prev())
                        >
                            <Icon name=IconName::ChevronLeft />
                        </button>
                        <span class="px-3 min-w-[11rem] text-center text-xs font-bold text-zinc-800 dark:text-dim-200" aria-live="polite">
                            {move || month.get().label()}
                        </span>
                        <button
                            type="button"
                            class=NAV_BTN
                            aria-label="Next month"
                            on:click=move |_| month.update(|m| *m = m.next())
                        >
                            <Icon name=IconName::ChevronRight />
                        </button>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <section class=format!("{CARD} lg:col-span-2 space-y-4")>
                    <div class="flex items-center justify-between border-b border-zinc-100 dark:border-dim-800 pb-3">
                        <h2 class="text-xs font-bold uppercase tracking-wider text-zinc-500">"Monthly overview"</h2>
                        <span class="text-xs font-semibold text-sakura-600 dark:text-sakura-400">
                            {logged_this_month} " / " {move || month.get().day_count()} " days logged"
                        </span>
                    </div>
                    <CalendarGrid month days selected />
                </section>

                <section class=format!("{CARD} space-y-4")>
                    <div class="border-b border-zinc-100 dark:border-dim-800 pb-3">
                        <p class="text-[11px] font-bold uppercase tracking-wider text-sakura-600 dark:text-sakura-400">
                            "Selected day"
                        </p>
                        <h2 class="text-lg font-bold font-jp text-zinc-900 dark:text-white">
                            {move || selected.get().format_ja()}
                        </h2>
                    </div>
                    {panel}
                </section>
            </div>

            <Heatmap title="Words added" unit="words" counts=word_counts palette=Palette::Sakura />
            <Heatmap
                title="Cards reviewed"
                unit="reviews"
                counts=state.review_log
                palette=Palette::Matcha
                empty_hint="Cards you review on the Review page show up here."
            />
        </div>
    }
}
