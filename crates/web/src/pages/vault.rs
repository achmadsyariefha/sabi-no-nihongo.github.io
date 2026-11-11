use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use sabi_core::DailyEntry;
use sabi_core::model::sort_newest_first;
use sabi_core::vault::{VaultFilter, tag_counts};

use crate::components::empty_state::EmptyState;
use crate::components::entry_card::EntryCard;
use crate::components::icon::{Icon, IconName};
use crate::services::base::app_href;
use crate::state::use_app_state;

const CHIP: &str = "shrink-0 px-3 py-1 rounded-full text-xs font-medium border transition-colors";
const CHIP_ON: &str = "bg-zinc-900 dark:bg-white border-zinc-900 dark:border-white text-white dark:text-dim-900";
const CHIP_OFF: &str = "bg-white dark:bg-dim-900 border-zinc-200 dark:border-dim-700 text-zinc-600 dark:text-dim-300 hover:border-zinc-400";

fn chip(active: bool) -> String {
    format!("{CHIP} {}", if active { CHIP_ON } else { CHIP_OFF })
}

#[component]
pub fn VaultPage() -> impl IntoView {
    let state = use_app_state();
    let navigate = use_navigate();
    let filter = RwSignal::new(VaultFilter::default());

    let tags = Memo::new(move |_| state.entries.with(|list| tag_counts(list)));
    let results = Memo::new(move |_| {
        let f = filter.get();
        state.entries.with(|list| {
            let mut found: Vec<DailyEntry> = f.apply(list).into_iter().cloned().collect();
            sort_newest_first(&mut found);
            found
        })
    });

    // navigate() adds the router base itself, so pass the bare path.
    let on_edit = Callback::new(move |id: String| {
        state.editing.set(Some(id));
        navigate("/", Default::default());
    });

    view! {
        <Show
            when=move || state.entries.with(|l| !l.is_empty())
            fallback=|| {
                view! {
                    <EmptyState
                        kanji="語彙"
                        title="No words yet"
                        message="Every word you log will be collected here, searchable by text and tag."
                    >
                        <A
                            href=app_href("/")
                            attr:class="inline-flex px-4 py-2 rounded-full bg-zinc-900 dark:bg-white text-white dark:text-dim-900 text-sm font-semibold"
                        >
                            "Go to Today"
                        </A>
                    </EmptyState>
                }
            }
        >
            <div class="space-y-5">
                <div class="flex flex-col sm:flex-row sm:items-end sm:justify-between gap-2">
                    <div>
                        <p class="text-xs font-semibold uppercase tracking-wider text-sakura-600 dark:text-sakura-400">"Word Vault"</p>
                        <h1 class="text-2xl font-bold text-zinc-900 dark:text-white">
                            {move || {
                                let n = state.stats.with(|s| s.total_entries);
                                if n == 1 { "1 word".to_owned() } else { format!("{n} words") }
                            }}
                        </h1>
                    </div>
                    <Show when=move || filter.with(|f| f.is_active())>
                        <p class="text-sm text-zinc-500 dark:text-dim-400">
                            {move || results.with(|r| r.len())}
                            " shown · "
                            <button
                                type="button"
                                class="font-semibold text-sakura-600 dark:text-sakura-400 hover:underline"
                                on:click=move |_| filter.set(VaultFilter::default())
                            >
                                "Clear filters"
                            </button>
                        </p>
                    </Show>
                </div>

                <div class="flex flex-col sm:flex-row gap-3">
                    <label class="relative flex-1">
                        <span class="sr-only">"Search words"</span>
                        <span class="absolute left-3.5 top-1/2 -translate-y-1/2 text-zinc-400">
                            <Icon name=IconName::Search />
                        </span>
                        <input
                            type="search"
                            class="w-full rounded-full border border-zinc-200 dark:border-dim-700 bg-white dark:bg-dim-900 pl-10 pr-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-sakura-300 dark:focus:ring-sakura-800"
                            placeholder="Search Japanese, reading, meaning, notes, tags…"
                            prop:value=move || filter.with(|f| f.query.clone())
                            on:input=move |ev| filter.update(|f| f.query = event_target_value(&ev))
                        />
                    </label>
                    <button
                        type="button"
                        class=move || format!("{} inline-flex items-center justify-center gap-1.5 py-2.5 text-sm", chip(filter.with(|f| f.favorites_only)))
                        aria-pressed=move || filter.with(|f| f.favorites_only).to_string()
                        on:click=move |_| filter.update(|f| f.favorites_only = !f.favorites_only)
                    >
                        <Icon name=IconName::Star class="w-4 h-4 fill-current text-amber-400" />
                        "Favorites"
                    </button>
                </div>

                <div class="flex gap-2 overflow-x-auto pb-1 -mx-4 px-4 sm:mx-0 sm:px-0 sm:flex-wrap">
                    <button
                        type="button"
                        class=move || chip(filter.with(|f| f.tag.is_none()))
                        on:click=move |_| filter.update(|f| f.tag = None)
                    >
                        "All tags"
                    </button>
                    <For each=move || tags.get() key=|(t, n)| (t.clone(), *n) let:item>
                        {
                            let (tag, n) = item;
                            let tag = StoredValue::new(tag);
                            let active = move || filter.with(|f| tag.with_value(|t| f.tag.as_ref() == Some(t)));
                            view! {
                                <button
                                    type="button"
                                    class=move || chip(active())
                                    aria-pressed=move || active().to_string()
                                    on:click=move |_| {
                                        let t = tag.get_value();
                                        filter.update(|f| f.tag = if f.tag.as_ref() == Some(&t) { None } else { Some(t) });
                                    }
                                >
                                    "#"
                                    {tag.get_value()}
                                    <span class="ml-1 opacity-60">{n}</span>
                                </button>
                            }
                        }
                    </For>
                </div>

                <Show
                    when=move || results.with(|r| !r.is_empty())
                    fallback=|| {
                        view! {
                            <p class="py-12 text-center text-sm text-zinc-500 dark:text-dim-400">"No words match these filters."</p>
                        }
                    }
                >
                    <div class="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
                        <For each=move || results.get() key=|e| (e.id.clone(), e.updated_at) let:entry>
                            <EntryCard entry compact=true on_edit />
                        </For>
                    </div>
                </Show>
            </div>
        </Show>
    }
}
