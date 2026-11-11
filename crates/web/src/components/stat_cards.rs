//! Streak, today's goal, and collection totals.

use leptos::prelude::*;
use sabi_core::DailyLimit;

use crate::components::icon::{Icon, IconName};
use crate::state::use_app_state;

const CARD: &str = "rounded-2xl border border-zinc-200/80 dark:border-dim-800 bg-white dark:bg-dim-900 p-4 sm:p-5";
const LABEL: &str = "text-[11px] font-semibold uppercase tracking-wider text-zinc-400 dark:text-dim-500";

fn days(n: u32) -> String {
    if n == 1 { "1 day".into() } else { format!("{n} days") }
}

#[component]
pub fn StatCards() -> impl IntoView {
    let state = use_app_state();
    let stats = state.stats;
    let limit = move || state.daily_limit();
    let goal_met = move || limit().is_met(stats.with(|s| s.today_count));
    let progress = move || {
        let count = stats.with(|s| s.today_count);
        match limit().count() {
            Some(n) => (count.min(n) * 100 / n).to_string(),
            None => if count > 0 { "100".into() } else { "0".into() },
        }
    };

    view! {
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 sm:gap-4">
            <section class=CARD>
                <p class=LABEL>"Streak"</p>
                <div class="mt-2 flex items-center gap-2 text-matcha-600 dark:text-matcha-400">
                    <Icon name=IconName::Flame class="w-6 h-6" />
                    <span class="text-3xl font-bold text-zinc-900 dark:text-white">{move || stats.with(|s| s.current)}</span>
                    <span class="text-sm text-zinc-500 dark:text-dim-400">
                        {move || if stats.with(|s| s.current) == 1 { "day" } else { "days" }}
                    </span>
                </div>
                <p class="mt-1 text-xs text-zinc-500 dark:text-dim-400">"Best: " {move || days(stats.with(|s| s.longest))}</p>
            </section>

            <section class=CARD>
                <div class="flex items-center justify-between gap-2">
                    <p class=LABEL>"Today"</p>
                    <select
                        aria-label="Daily goal"
                        class="text-xs rounded-lg border border-zinc-200 dark:border-dim-700 bg-white dark:bg-dim-950 px-2 py-1 text-zinc-600 dark:text-dim-300"
                        prop:value=move || limit().as_number().to_string()
                        on:change=move |ev| {
                            if let Some(l) = event_target_value(&ev).parse().ok().and_then(DailyLimit::from_number) {
                                state.set_daily_limit(l);
                            }
                        }
                    >
                        {DailyLimit::ALL
                            .iter()
                            .map(|l| view! { <option value=l.as_number().to_string()>{l.label()}</option> })
                            .collect_view()}
                    </select>
                </div>
                <div class="mt-2 flex items-baseline gap-1">
                    <span class="text-3xl font-bold text-zinc-900 dark:text-white">{move || stats.with(|s| s.today_count)}</span>
                    {move || limit().count().map(|n| view! { <span class="text-sm text-zinc-400">"/" {n}</span> })}
                    <Show when=goal_met>
                        <span class="ml-2 inline-flex items-center gap-1 text-xs font-semibold text-matcha-600 dark:text-matcha-400">
                            <Icon name=IconName::Trophy class="w-3.5 h-3.5" />
                            "Goal met"
                        </span>
                    </Show>
                </div>
                <div class="mt-2 h-1.5 rounded-full bg-zinc-100 dark:bg-dim-800 overflow-hidden">
                    <div
                        class="h-full rounded-full bg-sakura-500 transition-all"
                        style:width=move || format!("{}%", progress())
                    ></div>
                </div>
            </section>

            <section class=CARD>
                <p class=LABEL>"Collection"</p>
                <div class="mt-2 flex items-baseline gap-2">
                    <span class="text-3xl font-bold text-zinc-900 dark:text-white">{move || stats.with(|s| s.total_entries)}</span>
                    <span class="text-sm text-zinc-500 dark:text-dim-400">
                        {move || if stats.with(|s| s.total_entries) == 1 { "word" } else { "words" }}
                    </span>
                </div>
                <p class="mt-1 flex items-center gap-1 text-xs text-zinc-500 dark:text-dim-400">
                    <Icon name=IconName::Star class="w-3.5 h-3.5 fill-current text-amber-400" />
                    {move || {
                        let n = stats.with(|s| s.favorites);
                        if n == 1 { "1 favorite".to_owned() } else { format!("{n} favorites") }
                    }}
                </p>
            </section>
        </div>
    }
}
