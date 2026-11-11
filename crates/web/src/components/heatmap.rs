//! A year of daily activity as a GitHub-style grid.

use leptos::html::Div;
use leptos::prelude::*;
use sabi_core::activity::{DayCounts, Heatmap as Layout, level};

use crate::state::use_app_state;

const WEEKS: usize = 53;
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DAY_LABELS: [&str; 7] = ["", "Mon", "", "Wed", "", "Fri", ""];
/// Cell width (12px) plus gap (3px), for placing month labels.
const COLUMN_PX: usize = 15;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Palette {
    Sakura,
    Matcha,
}

impl Palette {
    // Full class names so Tailwind can find them.
    fn levels(self) -> [&'static str; 5] {
        match self {
            Palette::Sakura => [
                "bg-zinc-200/70 dark:bg-dim-800",
                "bg-sakura-200 dark:bg-sakura-900",
                "bg-sakura-300 dark:bg-sakura-700",
                "bg-sakura-400 dark:bg-sakura-500",
                "bg-sakura-600 dark:bg-sakura-400",
            ],
            Palette::Matcha => [
                "bg-zinc-200/70 dark:bg-dim-800",
                "bg-matcha-200 dark:bg-matcha-900",
                "bg-matcha-300 dark:bg-matcha-700",
                "bg-matcha-400 dark:bg-matcha-500",
                "bg-matcha-600 dark:bg-matcha-400",
            ],
        }
    }
}

#[component]
pub fn Heatmap(
    #[prop(into)] title: String,
    /// Plural noun for the counts, e.g. "words".
    unit: &'static str,
    #[prop(into)] counts: Signal<DayCounts>,
    palette: Palette,
    /// Shown under the grid when there is no activity at all.
    #[prop(optional)]
    empty_hint: Option<&'static str>,
) -> impl IntoView {
    let state = use_app_state();
    let layout = Memo::new(move |_| {
        let today = state.today.get();
        counts.with(|c| Layout::build(c, today, WEEKS))
    });
    let levels = palette.levels();

    // Show the newest weeks first when the grid is wider than the screen.
    let scroller = NodeRef::<Div>::new();
    Effect::new(move |_| {
        layout.track();
        if let Some(el) = scroller.get() {
            el.set_scroll_left(el.scroll_width());
        }
    });

    let unit_for = move |n: u32| if n == 1 { unit.trim_end_matches('s') } else { unit };

    view! {
        <section class="bg-paper dark:bg-dim-900 rounded-2xl border border-zinc-200/80 dark:border-dim-800 p-5 space-y-3">
            <div class="flex flex-wrap items-center justify-between gap-2">
                <h2 class="text-xs font-bold uppercase tracking-wider text-zinc-500">{title}</h2>
                <p class="text-xs text-zinc-500 dark:text-dim-400">
                    {move || {
                        layout
                            .with(|m| {
                                let days = if m.active_days == 1 { "day" } else { "days" };
                                view! {
                                    <span class="font-semibold text-zinc-800 dark:text-dim-200">{m.total}</span>
                                    " " {unit_for(m.total)} " on "
                                    <span class="font-semibold text-zinc-800 dark:text-dim-200">{m.active_days}</span>
                                    " " {days} " in the last year"
                                }
                            })
                    }}
                </p>
            </div>

            <div node_ref=scroller class="overflow-x-auto pb-1">
                <div class="inline-flex gap-2">
                    <div class="flex flex-col gap-[3px] pt-[18px] text-[9px] leading-[12px] text-zinc-400 dark:text-dim-500" aria-hidden="true">
                        {DAY_LABELS.map(|l| view! { <div class="h-3">{l}</div> }).collect_view()}
                    </div>
                    <div>
                        <div class="relative h-[15px] mb-[3px]" aria-hidden="true">
                            {move || {
                                layout
                                    .with(|m| {
                                        m.month_marks
                                            .iter()
                                            .map(|&(col, month)| {
                                                view! {
                                                    <span
                                                        class="absolute text-[9px] text-zinc-400 dark:text-dim-500"
                                                        style:left=format!("{}px", col * COLUMN_PX)
                                                    >
                                                        {MONTHS[month as usize - 1]}
                                                    </span>
                                                }
                                            })
                                            .collect_view()
                                    })
                            }}
                        </div>
                        <div class="flex gap-[3px]">
                            {move || {
                                let today = state.today.get_untracked();
                                layout
                                    .with(|m| {
                                        m.weeks
                                            .iter()
                                            .map(|week| {
                                                let cells = week
                                                    .iter()
                                                    .map(|cell| {
                                                        if cell.future {
                                                            return view! { <div class="w-3 h-3"></div> }.into_any();
                                                        }
                                                        let ring = if cell.day == today {
                                                            " ring-1 ring-zinc-500 dark:ring-dim-300"
                                                        } else {
                                                            ""
                                                        };
                                                        let class = format!(
                                                            "w-3 h-3 rounded-[3px] {}{ring}",
                                                            levels[level(cell.count)],
                                                        );
                                                        let tip = format!("{}: {} {}", cell.day, cell.count, unit_for(cell.count));
                                                        view! { <div class=class title=tip></div> }.into_any()
                                                    })
                                                    .collect_view();
                                                view! { <div class="flex flex-col gap-[3px]">{cells}</div> }
                                            })
                                            .collect_view()
                                    })
                            }}
                        </div>
                    </div>
                </div>
            </div>

            <div class="flex flex-wrap items-center justify-between gap-2 text-[10px] text-zinc-400 dark:text-dim-500">
                <span>
                    {move || {
                        empty_hint.filter(|_| layout.with(|m| m.total == 0)).map(|h| view! { <span class="text-xs">{h}</span> })
                    }}
                </span>
                <div class="flex items-center gap-1.5" aria-hidden="true">
                    <span>"Less"</span>
                    {levels.map(|c| view! { <div class=format!("w-3 h-3 rounded-[3px] {c}")></div> }).collect_view()}
                    <span>"More"</span>
                </div>
            </div>
        </section>
    }
}
