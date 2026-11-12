use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::A;
use sabi_core::DailyEntry;
use sabi_core::review::{self, Mark, ReviewFilter};
use sabi_core::srs::{self, Rating};
use wasm_bindgen::JsCast;

use crate::components::empty_state::EmptyState;
use crate::components::heatmap::{Heatmap, Palette};
use crate::components::icon::{Icon, IconName};
use crate::services::base::app_href;
use crate::services::{clock, speech};
use crate::state::use_app_state;

const TOOL_BTN: &str = "p-2 rounded-xl bg-white dark:bg-dim-800 border border-zinc-200 dark:border-dim-700 text-zinc-600 dark:text-dim-300 hover:bg-zinc-50 dark:hover:bg-dim-700 transition-colors";
const NAV_BTN: &str = "px-4 py-2.5 rounded-xl bg-white dark:bg-dim-800 border border-zinc-200 dark:border-dim-700 text-xs font-bold text-zinc-700 dark:text-dim-300 disabled:opacity-40 hover:bg-zinc-50 dark:hover:bg-dim-700 flex items-center gap-1.5";

fn filter_btn(active: bool) -> &'static str {
    if active {
        "flex items-center gap-1 px-2.5 py-1 rounded-lg font-semibold bg-white dark:bg-dim-700 text-zinc-900 dark:text-white shadow-sm"
    } else {
        "flex items-center gap-1 px-2.5 py-1 rounded-lg font-semibold text-zinc-500 dark:text-dim-400"
    }
}

#[component]
pub fn ReviewPage() -> impl IntoView {
    let state = use_app_state();
    let session = state.review;

    // Time used to order the Due queue. Updated on each answer, not on a
    // timer, so the current card never changes under the user.
    let now_ms = RwSignal::new(clock::now_ms());

    // Rebuild the deck only when the words, filter or shuffle change
    // (or, for Due, the schedule).
    let deck_key = Memo::new(move |_| session.with(|s| (s.filter, s.order.clone())));
    let deck = Memo::new(move |_| {
        let (filter, order) = deck_key.get();
        if filter == ReviewFilter::Due {
            let cutoff = clock::start_of_day_ms(state.today.get().next());
            let now = now_ms.get();
            return state.entries.with(|list| {
                state.srs.with(|s| srs::due_queue(list, s, now, cutoff).into_iter().cloned().collect::<Vec<DailyEntry>>())
            });
        }
        state.entries.with(|list| review::deck(list, filter, &order).into_iter().cloned().collect::<Vec<DailyEntry>>())
    });
    // In Due mode the current card is always the head of the queue.
    let index = Memo::new(move |_| {
        let n = deck.with(Vec::len);
        if deck_key.with(|k| k.0) == ReviewFilter::Due { 0 } else { session.with(|s| s.index_in(n)) }
    });
    let counts = Memo::new(move |_| {
        deck.with(|d| {
            let refs: Vec<&DailyEntry> = d.iter().collect();
            session.with(|s| s.counts(&refs))
        })
    });
    let flipped = RwSignal::new(false);
    Effect::new(move |_| {
        index.track();
        flipped.set(false);
    });

    let filter = move || session.with(|s| s.filter);
    let set_filter = move |f: ReviewFilter| session.update(|s| s.set_filter(f));
    let go_to = move |i: usize| {
        let n = deck.with_untracked(Vec::len);
        session.update(|s| s.go_to(i, n));
    };
    let mark = move |m: Mark| {
        let i = index.get_untracked();
        let id = deck.with_untracked(|d| d.get(i).map(|e| e.id.clone()));
        if let Some(id) = id {
            let n = deck.with_untracked(Vec::len);
            state.mark_card(&id, m, n);
            if index.get_untracked() >= n {
                state.fire_confetti();
            }
        }
    };
    let rate = move |r: Rating| {
        let id = deck.with_untracked(|d| d.first().map(|e| e.id.clone()));
        if let Some(id) = id {
            state.rate_card(&id, r);
            now_ms.set(clock::now_ms());
            flipped.set(false);
            if deck.with_untracked(Vec::is_empty) {
                state.fire_confetti();
            }
        }
    };
    // 1 / 2 answer in either mode.
    let answer = move |known: bool| {
        if filter() == ReviewFilter::Due {
            rate(if known { Rating::Pass } else { Rating::Fail });
        } else {
            mark(if known { Mark::Known } else { Mark::NeedReview });
        }
    };

    // Keyboard: Space flips, 1 = need review, 2 = remembered, arrows move.
    let handle = window_event_listener(ev::keydown, move |e| {
        if state.backup_open.get_untracked() || e.ctrl_key() || e.meta_key() || e.alt_key() || e.repeat() {
            return;
        }
        let key = e.key();
        if let Some(el) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            let tag = el.tag_name();
            if matches!(tag.as_str(), "INPUT" | "TEXTAREA" | "SELECT") {
                return;
            }
            // Space or Enter on a focused button already clicks it.
            if tag == "BUTTON" && (key == " " || key == "Enter") {
                return;
            }
        }
        let due = filter() == ReviewFilter::Due;
        match key.as_str() {
            " " => {
                e.prevent_default();
                flipped.update(|f| *f = !*f);
            }
            "1" => answer(false),
            "2" => answer(true),
            "ArrowLeft" if !due => go_to(index.get_untracked().saturating_sub(1)),
            "ArrowRight" if !due => go_to(index.get_untracked() + 1),
            _ => {}
        }
    });
    on_cleanup(move || handle.remove());
    let shuffle = move |_| {
        deck.with_untracked(|d| {
            let refs: Vec<&DailyEntry> = d.iter().collect();
            session.update(|s| s.shuffle(&refs, |n| (js_sys::Math::random() * n as f64) as usize % n));
        });
    };
    let restart = move |_| {
        if window().confirm_with_message("Start today's review over? Your marks for today will be cleared.").unwrap_or(false) {
            session.update(|s| s.restart());
        }
    };

    let header = view! {
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
            <div>
                <h1 class="text-2xl font-bold font-jp-serif text-zinc-900 dark:text-white flex items-center gap-2">
                    <Icon name=IconName::Layers class="w-6 h-6 text-sakura-500" />
                    "Review"
                </h1>
                <p class="text-xs text-zinc-500 dark:text-dim-400 mt-0.5">
                    {move || {
                        if filter() == ReviewFilter::Due {
                            "Spaced repetition: words you remember come back less and less often."
                        } else {
                            "Flip each card, then say whether you remembered it."
                        }
                    }}
                </p>
            </div>
            <div class="flex items-center gap-2">
                <div
                    class="flex items-center bg-zinc-100 dark:bg-dim-800 p-1 rounded-xl border border-zinc-200 dark:border-dim-700 text-xs"
                    role="group"
                    aria-label="Which words to review"
                >
                    <button
                        type="button"
                        class=move || filter_btn(filter() == ReviewFilter::All)
                        aria-pressed=move || (filter() == ReviewFilter::All).to_string()
                        on:click=move |_| set_filter(ReviewFilter::All)
                    >
                        "All (" {move || state.stats.with(|s| s.total_entries)} ")"
                    </button>
                    <button
                        type="button"
                        class=move || filter_btn(filter() == ReviewFilter::Favorites)
                        aria-pressed=move || (filter() == ReviewFilter::Favorites).to_string()
                        on:click=move |_| set_filter(ReviewFilter::Favorites)
                    >
                        <Icon name=IconName::Star class="w-3 h-3 fill-current text-amber-400" />
                        "Favorites (" {move || state.stats.with(|s| s.favorites)} ")"
                    </button>
                    <button
                        type="button"
                        class=move || filter_btn(filter() == ReviewFilter::Due)
                        aria-pressed=move || (filter() == ReviewFilter::Due).to_string()
                        title="Spaced repetition: words due today"
                        on:click=move |_| {
                            now_ms.set(clock::now_ms());
                            set_filter(ReviewFilter::Due);
                        }
                    >
                        <Icon name=IconName::RotateCw class="w-3 h-3" />
                        "Due (" {move || state.due_count.get()} ")"
                    </button>
                </div>
                <Show when=move || filter() != ReviewFilter::Due>
                    <button type="button" class=TOOL_BTN title="Shuffle the deck" aria-label="Shuffle the deck" on:click=shuffle>
                        <Icon name=IconName::Shuffle />
                    </button>
                    <button
                        type="button"
                        class=TOOL_BTN
                        title="Start today's review over"
                        aria-label="Start today's review over"
                        on:click=restart
                    >
                        <Icon name=IconName::RotateCcw />
                    </button>
                </Show>
            </div>
        </div>
    };

    let progress = move || {
        let n = deck.with(Vec::len);
        let (known, need) = counts.get();
        let done = known + need;
        let pct = (done * 100).checked_div(n).unwrap_or(0);
        let pos = (index.get() + 1).min(n);
        view! {
            <div class="space-y-1.5">
                <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 text-xs font-semibold text-zinc-500 dark:text-dim-400">
                    <div class="flex items-center gap-3">
                        <span>"Card " {pos} " of " {n}</span>
                        <span class="text-matcha-600 dark:text-matcha-400">"✓ " {known} " remembered"</span>
                        <span class="text-rose-600 dark:text-rose-400">"✗ " {need} " to review"</span>
                    </div>
                    <span>{pct} "% reviewed"</span>
                </div>
                <div class="w-full h-2 rounded-full bg-zinc-100 dark:bg-dim-800 overflow-hidden">
                    <div
                        class="h-full bg-gradient-to-r from-sakura-500 to-sakura-400 transition-all duration-300"
                        style:width=format!("{pct}%")
                    ></div>
                </div>
                <p class="text-[11px] text-zinc-400 dark:text-dim-500">
                    "Progress saves automatically and starts over at midnight."
                </p>
            </div>
        }
    };

    let due_progress = move || {
        let left = deck.with(Vec::len);
        let since = clock::start_of_day_ms(state.today.get());
        let done = state.srs.with(|s| srs::reviewed_since(s, since));
        let pct = (done * 100).checked_div(done + left).unwrap_or(100);
        view! {
            <div class="space-y-1.5">
                <div class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 text-xs font-semibold text-zinc-500 dark:text-dim-400">
                    <span>{left} " due · " {done} " reviewed today"</span>
                    <span>{pct} "% done"</span>
                </div>
                <div class="w-full h-2 rounded-full bg-zinc-100 dark:bg-dim-800 overflow-hidden">
                    <div
                        class="h-full bg-gradient-to-r from-indigoPaper-500 to-indigoPaper-400 transition-all duration-300"
                        style:width=format!("{pct}%")
                    ></div>
                </div>
                <p class="text-[11px] text-zinc-400 dark:text-dim-500">
                    "A word you miss comes back in 10 minutes. One you remember waits 1 day, then 6, then longer."
                </p>
            </div>
        }
    };

    let main = move || {
        let n = deck.with(Vec::len);
        if n == 0 && filter() == ReviewFilter::Due && state.entries.with(|l| !l.is_empty()) {
            let next = state.entries.with(|list| state.srs.with(|s| srs::next_due(list, s)));
            let next_day = next.map(|ms| clock::date_of(ms).format_ja()).unwrap_or_default();
            return view! {
                <div class="space-y-6">
                    {due_progress}
                    <section class="rounded-3xl border-2 border-zinc-200/90 dark:border-dim-800 bg-white dark:bg-dim-900 p-8 text-center space-y-3">
                        <p class="font-jp-serif text-5xl font-bold text-zinc-900 dark:text-white">"完了"</p>
                        <h2 class="text-xl font-semibold text-zinc-900 dark:text-white">"All caught up"</h2>
                        <p class="text-sm text-zinc-600 dark:text-dim-300">
                            "Nothing else is due today. Next review: " <span class="font-jp">{next_day}</span> "."
                        </p>
                    </section>
                </div>
            }
            .into_any();
        }
        if n == 0 {
            if filter() == ReviewFilter::Favorites {
                return view! {
                    <EmptyState kanji="☆" title="No favorites yet" message="Star some words to review just those, or review everything.">
                        <button
                            type="button"
                            class="px-4 py-2 rounded-full bg-sakura-500 hover:bg-sakura-600 text-white text-sm font-semibold"
                            on:click=move |_| set_filter(ReviewFilter::All)
                        >
                            "Review all words"
                        </button>
                    </EmptyState>
                }
                .into_any();
            }
            return view! {
                <EmptyState kanji="復習" title="No cards to review" message="Log some words first, then review them here as flashcards.">
                    <A
                        href=app_href("/")
                        attr:class="inline-flex px-4 py-2 rounded-full bg-zinc-900 dark:bg-white text-white dark:text-dim-900 text-sm font-semibold"
                    >
                        "Go to Today"
                    </A>
                </EmptyState>
            }
            .into_any();
        }

        let i = index.get();
        if filter() == ReviewFilter::Due {
            let card = deck.with_untracked(|d| d[0].clone());
            let schedule = state.srs.with(|s| s.get(&card.id).cloned());
            let info = match schedule {
                None => "New word".to_owned(),
                Some(c) if c.interval == 0 => "Learning again".to_owned(),
                Some(c) if c.interval == 1 => "Last gap: 1 day".to_owned(),
                Some(c) => format!("Last gap: {} days", c.interval),
            };
            return view! {
                <div class="space-y-6">
                    {due_progress}
                    <Flashcard card flipped current_mark=None info />
                    <div class="flex items-center gap-2 w-full sm:justify-end">
                        <MarkButton mark=Mark::NeedReview current=None on_click=Callback::new(move |_| rate(Rating::Fail)) />
                        <MarkButton mark=Mark::Known current=None on_click=Callback::new(move |_| rate(Rating::Pass)) />
                    </div>
                    {shortcut_hint(true)}
                </div>
            }
            .into_any();
        }
        if i >= n {
            let (known, need) = counts.get_untracked();
            return view! {
                <div class="space-y-6">
                    {progress}
                    <section class="rounded-3xl border-2 border-zinc-200/90 dark:border-dim-800 bg-white dark:bg-dim-900 p-8 text-center space-y-4">
                        <p class="font-jp-serif text-5xl font-bold text-zinc-900 dark:text-white">"完了"</p>
                        <h2 class="text-xl font-semibold text-zinc-900 dark:text-white">"Review complete"</h2>
                        <p class="text-sm text-zinc-600 dark:text-dim-300">
                            "You remembered " <b>{known}</b> " of " {n} {if n == 1 { " card" } else { " cards" }}
                            {(need > 0).then(|| format!(", and marked {need} to review again"))} "."
                        </p>
                        <div class="flex flex-col sm:flex-row items-center justify-center gap-2 pt-2">
                            <button type="button" class=NAV_BTN on:click=move |_| go_to(n - 1)>
                                <Icon name=IconName::ChevronLeft />
                                "Back to the last card"
                            </button>
                            <button
                                type="button"
                                class="px-4 py-2.5 rounded-xl bg-sakura-500 hover:bg-sakura-600 text-white text-xs font-bold flex items-center gap-1.5"
                                on:click=move |_| session.update(|s| s.restart())
                            >
                                <Icon name=IconName::RotateCw />
                                "Review again"
                            </button>
                        </div>
                    </section>
                </div>
            }
            .into_any();
        }

        let card = deck.with_untracked(|d| d[i].clone());
        let current_mark = session.with(|s| s.mark_of(&card.id));
        let at_start = i == 0;
        view! {
            <div class="space-y-6">
                {progress}
                <Flashcard card=card.clone() flipped current_mark />
                <div class="flex flex-col sm:flex-row items-center justify-between gap-3">
                    <div class="flex items-center gap-2 w-full sm:w-auto justify-between sm:justify-start">
                        <button type="button" class=NAV_BTN disabled={at_start} on:click=move |_| go_to(i.saturating_sub(1))>
                            <Icon name=IconName::ChevronLeft />
                            "Previous"
                        </button>
                        <button type="button" class=NAV_BTN on:click=move |_| go_to(i + 1)>
                            "Skip"
                            <Icon name=IconName::ChevronRight />
                        </button>
                    </div>
                    <div class="flex items-center gap-2 w-full sm:w-auto">
                        <MarkButton
                            mark=Mark::NeedReview
                            current=current_mark
                            count=Signal::derive(move || counts.get().1)
                            on_click=Callback::new(move |_| mark(Mark::NeedReview))
                        />
                        <MarkButton
                            mark=Mark::Known
                            current=current_mark
                            count=Signal::derive(move || counts.get().0)
                            on_click=Callback::new(move |_| mark(Mark::Known))
                        />
                    </div>
                </div>
                {shortcut_hint(false)}
            </div>
        }
        .into_any()
    };

    view! {
        <div class="max-w-2xl mx-auto space-y-6">
            {header}
            {main}
            <Heatmap title="Cards reviewed" unit="reviews" counts=state.review_log palette=Palette::Matcha />
        </div>
    }
}

#[component]
fn MarkButton(
    mark: Mark,
    current: Option<Mark>,
    /// Shown in brackets after the label.
    #[prop(optional, into)]
    count: Option<Signal<usize>>,
    on_click: Callback<()>,
) -> impl IntoView {
    let active = current == Some(mark);
    let (class, icon, label) = match (mark, active) {
        (Mark::NeedReview, true) => ("bg-rose-500 text-white border-rose-600 ring-2 ring-rose-300", IconName::CircleX, "Need review"),
        (Mark::NeedReview, false) => (
            "bg-rose-50 dark:bg-rose-950/40 border-rose-200 dark:border-rose-900/60 text-rose-700 dark:text-rose-300 hover:bg-rose-100",
            IconName::CircleX,
            "Need review",
        ),
        (Mark::Known, true) => ("bg-matcha-500 text-white border-matcha-600 ring-2 ring-matcha-300", IconName::CircleCheck, "I remembered"),
        (Mark::Known, false) => (
            "bg-matcha-50 dark:bg-matcha-900/40 border-matcha-200 dark:border-matcha-900/60 text-matcha-700 dark:text-matcha-300 hover:bg-matcha-100",
            IconName::CircleCheck,
            "I remembered",
        ),
    };
    view! {
        <button
            type="button"
            class=format!("flex-1 sm:flex-none px-4 py-2.5 rounded-xl border text-xs font-bold flex items-center justify-center gap-1.5 transition-all {class}")
            aria-pressed=active.to_string()
            on:click=move |_| on_click.run(())
        >
            <Icon name=icon />
            {label}
            {count.map(|c| view! { " (" {c} ")" })}
        </button>
    }
}

#[component]
fn Flashcard(
    card: DailyEntry,
    flipped: RwSignal<bool>,
    current_mark: Option<Mark>,
    /// Extra note in the header, e.g. the card's SM-2 state.
    #[prop(optional, into)]
    info: Option<String>,
) -> impl IntoView {
    let speaking = RwSignal::new(false);
    let japanese = StoredValue::new(card.japanese.clone());
    let speak = move |_| {
        if speaking.get_untracked() {
            return;
        }
        speaking.set(true);
        japanese.with_value(|jp| speech::speak(jp, move || speaking.set(false)));
    };
    let no_speech = !speech::is_supported();

    let badge = current_mark.map(|m| match m {
        Mark::Known => view! {
            <span class="px-2 py-0.5 rounded-full bg-matcha-100 dark:bg-matcha-900/60 text-matcha-700 dark:text-matcha-300 font-bold text-[10px]">
                "Remembered ✓"
            </span>
        },
        Mark::NeedReview => view! {
            <span class="px-2 py-0.5 rounded-full bg-rose-100 dark:bg-rose-950/60 text-rose-700 dark:text-rose-300 font-bold text-[10px]">
                "Needs review ✗"
            </span>
        },
    });

    let front = {
        let card = card.clone();
        move || {
            view! {
                <span class="block font-jp text-4xl sm:text-5xl font-bold text-zinc-900 dark:text-white tracking-wide break-words">
                    {card.japanese.clone()}
                </span>
                {(!card.reading.is_empty())
                    .then(|| view! { <span class="block mt-3 font-jp text-base font-medium text-sakura-600 dark:text-sakura-400">{card.reading.clone()}</span> })}
            }
        }
    };
    let back = {
        let card = card.clone();
        move || {
            view! {
                <span class="block text-xs uppercase font-bold tracking-wider text-zinc-400">"Meaning"</span>
                <span class="block mt-1 text-2xl sm:text-3xl font-bold text-zinc-900 dark:text-white">{card.translation.clone()}</span>
                {card
                    .notes
                    .clone()
                    .map(|n| {
                        view! {
                            <span class="block mt-4 p-3 rounded-xl bg-amber-50/70 dark:bg-amber-950/30 border border-amber-200/50 dark:border-amber-900/40 text-xs text-amber-900 dark:text-amber-200 max-w-md mx-auto whitespace-pre-line">
                                {n}
                            </span>
                        }
                    })}
                {card
                    .example_sentence
                    .clone()
                    .map(|ex| {
                        view! {
                            <span class="block mt-3 p-3 rounded-xl bg-zinc-50 dark:bg-dim-800/60 border border-zinc-100 dark:border-dim-800 text-xs max-w-md mx-auto">
                                <span class="block font-jp font-semibold text-zinc-800 dark:text-dim-200">{ex.japanese}</span>
                                {(!ex.translation.is_empty())
                                    .then(|| view! { <span class="block mt-1 text-zinc-500 italic">{ex.translation}</span> })}
                            </span>
                        }
                    })}
            }
        }
    };

    view! {
        <section class="rounded-3xl border-2 border-zinc-200/90 dark:border-dim-800 bg-white dark:bg-dim-900 p-5 sm:p-8 flex flex-col gap-4 min-h-[320px] sm:min-h-[360px]">
            <div class="flex items-center justify-between gap-2 text-xs text-zinc-400">
                <div class="flex items-center gap-2">
                    <span class="font-semibold uppercase tracking-wider">{move || if flipped.get() { "Back" } else { "Front" }}</span>
                    {badge}
                    {info
                        .map(|i| {
                            view! {
                                <span class="px-2 py-0.5 rounded-full bg-indigoPaper-50 dark:bg-dim-800 text-indigoPaper-600 dark:text-indigoPaper-300 font-semibold text-[10px]">
                                    {i}
                                </span>
                            }
                        })}
                </div>
                {card.is_favorite.then(|| view! { <Icon name=IconName::Star class="w-4 h-4 fill-current text-amber-400" /> })}
            </div>
            <button
                type="button"
                class="flex-1 w-full rounded-2xl text-center py-6 cursor-pointer select-none hover:bg-zinc-50/60 dark:hover:bg-dim-800/30 focus:outline-none focus-visible:ring-2 focus-visible:ring-sakura-300"
                aria-label=move || if flipped.get() { "Show the front" } else { "Show the meaning" }
                on:click=move |_| flipped.update(|f| *f = !*f)
            >
                {move || if flipped.get() { back().into_any() } else { front().into_any() }}
            </button>
            <div class="flex items-center justify-between gap-2 pt-3 border-t border-zinc-100 dark:border-dim-800 text-xs">
                <ul class="flex flex-wrap gap-1">
                    {card
                        .tags
                        .iter()
                        .map(|t| {
                            view! {
                                <li class="text-[10px] px-2 py-0.5 rounded bg-zinc-100 dark:bg-dim-800 text-zinc-600 dark:text-dim-400">
                                    "#"{t.clone()}
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
                <div class="flex items-center gap-1 shrink-0">
                    <button
                        type="button"
                        class="p-2 rounded-xl bg-sakura-50 dark:bg-sakura-950/60 text-sakura-600 dark:text-sakura-400 hover:scale-105 transition-transform disabled:opacity-40"
                        title="Listen to pronunciation"
                        aria-label="Listen to pronunciation"
                        disabled={no_speech}
                        on:click=speak
                    >
                        <span class=move || if speaking.get() { "block animate-pulse" } else { "block" }>
                            <Icon name=IconName::Volume />
                        </span>
                    </button>
                    <button
                        type="button"
                        class="flex items-center gap-1 px-2 py-1.5 rounded-lg font-semibold text-sakura-600 dark:text-sakura-400 hover:bg-sakura-50 dark:hover:bg-dim-800"
                        on:click=move |_| flipped.update(|f| *f = !*f)
                    >
                        <Icon name=IconName::RotateCw class="w-3.5 h-3.5" />
                        "Flip"
                    </button>
                </div>
            </div>
        </section>
    }
}

/// Keyboard shortcuts, shown on larger screens only.
fn shortcut_hint(due: bool) -> impl IntoView {
    view! {
        <p class="hidden sm:block text-center text-[11px] text-zinc-400 dark:text-dim-500">
            "Keys: " <kbd class="font-sans font-semibold">"Space"</kbd> " flip · " <kbd class="font-sans font-semibold">"1"</kbd>
            " need review · " <kbd class="font-sans font-semibold">"2"</kbd> " remembered"
            {(!due).then(|| view! { " · " <kbd class="font-sans font-semibold">"← →"</kbd> " move" })}
        </p>
    }
}
