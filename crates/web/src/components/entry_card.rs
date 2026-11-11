//! One journal entry, full size (Today) or compact (Vault).

use leptos::prelude::*;
use sabi_core::DailyEntry;

use crate::components::icon::{Icon, IconName};
use crate::services::speech;
use crate::state::use_app_state;

const ICON_BTN: &str = "p-2 rounded-full text-zinc-500 dark:text-dim-400 hover:bg-zinc-100 dark:hover:bg-dim-800 hover:text-zinc-900 dark:hover:text-dim-100 transition-colors disabled:opacity-40";

#[component]
pub fn EntryCard(
    entry: DailyEntry,
    /// Smaller layout for grids.
    #[prop(optional)]
    compact: bool,
    /// Called with the entry id when "Edit" is pressed.
    #[prop(into)]
    on_edit: Callback<String>,
) -> impl IntoView {
    let state = use_app_state();
    let speaking = RwSignal::new(false);

    let id = StoredValue::new(entry.id.clone());
    let japanese = StoredValue::new(entry.japanese.clone());
    let is_favorite = entry.is_favorite;
    let no_speech = !speech::is_supported();

    let speak = move |_| {
        if speaking.get_untracked() {
            return;
        }
        speaking.set(true);
        japanese.with_value(|jp| speech::speak(jp, move || speaking.set(false)));
    };
    let toggle_favorite = move |_| id.with_value(|id| state.toggle_favorite(id));
    let edit = move |_| on_edit.run(id.get_value());
    let delete = move |_| {
        let msg = japanese.with_value(|jp| format!("Delete \u{201c}{jp}\u{201d}? This can't be undone."));
        if window().confirm_with_message(&msg).unwrap_or(false) {
            id.with_value(|id| state.delete(id));
        }
    };

    let actions = view! {
        <div class="flex items-center gap-0.5">
            <button
                type="button"
                class=ICON_BTN
                title="Listen to pronunciation"
                aria-label="Listen to pronunciation"
                disabled={no_speech}
                on:click=speak
            >
                <span class=move || if speaking.get() { "block text-sakura-500 animate-pulse" } else { "block" }>
                    <Icon name=IconName::Volume />
                </span>
            </button>
            <button
                type="button"
                class=ICON_BTN
                title=if is_favorite { "Remove from favorites" } else { "Mark as favorite" }
                aria-pressed=is_favorite.to_string()
                on:click=toggle_favorite
            >
                <Icon
                    name=IconName::Star
                    class=if is_favorite { "w-4 h-4 fill-current text-amber-400" } else { "w-4 h-4" }
                />
            </button>
            <button type="button" class=ICON_BTN title="Edit" aria-label="Edit" on:click=edit>
                <Icon name=IconName::PenLine />
            </button>
            <button type="button" class=ICON_BTN title="Delete" aria-label="Delete" on:click=delete>
                <Icon name=IconName::Trash />
            </button>
        </div>
    };

    let tags = (!entry.tags.is_empty()).then(|| {
        view! {
            <ul class="flex flex-wrap gap-1.5">
                {entry
                    .tags
                    .iter()
                    .map(|t| {
                        view! {
                            <li class="px-2 py-0.5 rounded-full text-[11px] font-medium bg-zen-100 dark:bg-dim-800 text-zen-700 dark:text-dim-300">
                                "#"{t.clone()}
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        }
    });

    let date = entry.date.format_ja();

    if compact {
        return view! {
            <article class="flex flex-col gap-3 rounded-2xl border border-zinc-200/80 dark:border-dim-800 bg-white dark:bg-dim-900 p-5">
                <div class="flex items-start justify-between gap-2">
                    <div class="min-w-0">
                        <p class="font-jp-serif text-2xl font-bold text-zinc-900 dark:text-white break-words">{entry.japanese.clone()}</p>
                        <p class="font-jp text-sm text-sakura-600 dark:text-sakura-400">{entry.reading.clone()}</p>
                    </div>
                    {is_favorite.then(|| view! { <Icon name=IconName::Star class="w-4 h-4 shrink-0 mt-2 fill-current text-amber-400" /> })}
                </div>
                <p class="text-sm text-zinc-700 dark:text-dim-200 line-clamp-2">{entry.translation.clone()}</p>
                {tags}
                <div class="mt-auto flex items-center justify-between pt-2 border-t border-zinc-100 dark:border-dim-800">
                    <span class="font-jp text-[11px] text-zinc-400 dark:text-dim-500">{date}</span>
                    {actions}
                </div>
            </article>
        }
        .into_any();
    }

    let notes = entry.notes.clone().map(|n| {
        view! {
            <div class="rounded-xl bg-paper dark:bg-dim-800/60 px-4 py-3">
                <p class="text-[11px] font-semibold uppercase tracking-wider text-zinc-400 dark:text-dim-500">"Notes"</p>
                <p class="mt-1 text-sm text-zinc-700 dark:text-dim-200 whitespace-pre-line">{n}</p>
            </div>
        }
    });
    let example = entry.example_sentence.clone().map(|ex| {
        view! {
            <div class="rounded-xl border border-indigoPaper-100 dark:border-dim-800 bg-indigoPaper-50/60 dark:bg-dim-800/40 px-4 py-3">
                <p class="text-[11px] font-semibold uppercase tracking-wider text-indigoPaper-600 dark:text-indigoPaper-300">"Example"</p>
                <p class="mt-1 font-jp text-base text-zinc-900 dark:text-white">{ex.japanese}</p>
                {ex.reading.map(|r| view! { <p class="font-jp text-xs text-zinc-500 dark:text-dim-400">{r}</p> })}
                {(!ex.translation.is_empty()).then(|| view! { <p class="mt-1 text-sm italic text-zinc-600 dark:text-dim-300">{ex.translation}</p> })}
            </div>
        }
    });

    view! {
        <article class="rounded-3xl border border-zinc-200/80 dark:border-dim-800 bg-white dark:bg-dim-900 p-5 sm:p-8 space-y-5">
            <div class="flex items-center justify-between gap-2">
                <span class="font-jp text-xs font-medium text-zinc-400 dark:text-dim-500">{date}</span>
                {actions}
            </div>
            <div class="text-center py-2 sm:py-4">
                <p class="font-jp-serif text-5xl sm:text-6xl font-bold text-zinc-900 dark:text-white break-words leading-tight">
                    {entry.japanese.clone()}
                </p>
                <p class="mt-3 font-jp text-lg text-sakura-600 dark:text-sakura-400">{entry.reading.clone()}</p>
                {entry.romaji.clone().map(|r| view! { <p class="text-sm text-zinc-400 dark:text-dim-500">{r}</p> })}
                <p class="mt-4 text-lg sm:text-xl text-zinc-800 dark:text-dim-100">{entry.translation.clone()}</p>
            </div>
            {notes}
            {example}
            {tags}
        </article>
    }
    .into_any()
}
