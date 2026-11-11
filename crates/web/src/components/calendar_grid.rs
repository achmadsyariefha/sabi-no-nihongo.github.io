//! Month grid: one button per day, showing the first word logged that day.

use std::collections::BTreeMap;

use leptos::prelude::*;
use sabi_core::DateKey;
use sabi_core::calendar::{DaySummary, Month};

use crate::components::icon::{Icon, IconName};
use crate::state::use_app_state;

const WEEKDAYS: [(&str, &str); 7] =
    [("Sun", "日"), ("Mon", "月"), ("Tue", "火"), ("Wed", "水"), ("Thu", "木"), ("Fri", "金"), ("Sat", "土")];

const CELL: &str = "h-16 sm:h-20 p-1.5 rounded-xl text-left flex flex-col justify-between transition-all border";
const SELECTED: &str = "ring-2 ring-sakura-500 border-sakura-400 bg-sakura-50/50 dark:bg-sakura-950/30";
const LOGGED: &str = "bg-gradient-to-br from-white to-zinc-50 dark:from-dim-800/80 dark:to-dim-800 border-zinc-200 dark:border-dim-700 hover:border-sakura-300";
const EMPTY: &str = "bg-zinc-50/60 dark:bg-dim-900/50 border-dashed border-zinc-200 dark:border-dim-800 hover:bg-zinc-100 dark:hover:bg-dim-800/60";

#[component]
pub fn CalendarGrid(
    #[prop(into)] month: Signal<Month>,
    #[prop(into)] days: Signal<BTreeMap<DateKey, DaySummary>>,
    selected: RwSignal<DateKey>,
) -> impl IntoView {
    let state = use_app_state();

    view! {
        <div class="grid grid-cols-7 gap-1 text-center text-[11px] font-bold text-zinc-400 dark:text-dim-500 pb-1">
            {WEEKDAYS
                .iter()
                .enumerate()
                .map(|(i, (en, ja))| {
                    let tone = match i {
                        0 => "text-rose-400",
                        6 => "text-blue-400",
                        _ => "",
                    };
                    view! {
                        <div class=tone>
                            <span class="hidden sm:inline">{*en} " "</span>
                            <span class="font-jp">{*ja}</span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
        <div class="grid grid-cols-7 gap-1.5 sm:gap-2">
            {move || {
                let today = state.today.get();
                let summaries = days.get();
                month
                    .get()
                    .cells()
                    .into_iter()
                    .map(|cell| {
                        let Some(day) = cell else {
                            return view! { <div class="h-16 sm:h-20 rounded-xl bg-zinc-50/40 dark:bg-dim-950/20"></div> }
                                .into_any();
                        };
                        let summary = summaries.get(&day).cloned();
                        let logged = summary.is_some();
                        let class = move || {
                            let tone = if selected.get() == day {
                                SELECTED
                            } else if logged {
                                LOGGED
                            } else {
                                EMPTY
                            };
                            format!("{CELL} {tone}")
                        };
                        let num_class = if day == today {
                            "text-xs font-bold w-5 h-5 flex items-center justify-center rounded-full bg-sakura-500 text-white"
                        } else {
                            "text-xs font-bold w-5 h-5 flex items-center justify-center rounded-full text-zinc-700 dark:text-dim-300"
                        };
                        let label = match &summary {
                            Some(s) if s.count == 1 => format!("{}, 1 word", day.format_ja()),
                            Some(s) => format!("{}, {} words", day.format_ja(), s.count),
                            None => format!("{}, no words", day.format_ja()),
                        };
                        let fav = summary.as_ref().is_some_and(|s| s.any_favorite);
                        let more = summary.as_ref().filter(|s| s.count > 1).map(|s| format!("+{}", s.count - 1));
                        let body = summary.map(|s| {
                            view! {
                                <div class="overflow-hidden w-full">
                                    <p class="text-xs font-bold font-jp text-zinc-900 dark:text-white truncate">
                                        {s.first_japanese}
                                    </p>
                                    <p class="text-[10px] text-zinc-500 dark:text-dim-400 truncate hidden sm:block">
                                        {s.first_translation}
                                    </p>
                                </div>
                            }
                        });
                        view! {
                            <button
                                type="button"
                                class=class
                                aria-label=label
                                aria-pressed=move || (selected.get() == day).to_string()
                                on:click=move |_| selected.set(day)
                            >
                                <div class="flex items-center justify-between gap-0.5 w-full">
                                    <span class=num_class>{day.day()}</span>
                                    <span class="flex items-center gap-0.5">
                                        {more
                                            .map(|m| {
                                                view! {
                                                    <span class="text-[9px] font-bold text-sakura-600 dark:text-sakura-400">{m}</span>
                                                }
                                            })}
                                        {fav
                                            .then(|| {
                                                view! { <Icon name=IconName::Star class="w-3 h-3 text-rose-500 fill-rose-500" /> }
                                            })}
                                    </span>
                                </div>
                                {body}
                            </button>
                        }
                            .into_any()
                    })
                    .collect_view()
            }}
        </div>
    }
}
