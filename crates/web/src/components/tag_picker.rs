//! Preset + saved custom tags as toggle chips, and an input for new tags.

use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use sabi_core::EntryDraft;
use sabi_core::tags::{self, PRESETS};

use crate::components::icon::{Icon, IconName};
use crate::state::use_app_state;

const CHIP: &str = "inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium border transition-colors";
const CHIP_ON: &str = "bg-sakura-500 border-sakura-500 text-white";
const CHIP_OFF: &str = "bg-white dark:bg-dim-900 border-zinc-200 dark:border-dim-700 text-zinc-600 dark:text-dim-300 hover:border-sakura-300 dark:hover:border-sakura-700";

#[component]
pub fn TagPicker(draft: RwSignal<EntryDraft>) -> impl IntoView {
    let state = use_app_state();
    let new_tag = RwSignal::new(String::new());
    let hovered = RwSignal::new(None::<&'static str>);

    let is_on = move |tag: &str| draft.with(|d| d.tags.iter().any(|t| t == tag));
    let toggle = move |tag: String| {
        draft.update(|d| match d.tags.iter().position(|t| *t == tag) {
            Some(i) => {
                d.tags.remove(i);
            }
            None => d.tags.push(tag),
        })
    };
    let add_new = move || {
        if let Some(tag) = tags::normalize(&new_tag.get_untracked()) {
            draft.update(|d| {
                if !d.tags.contains(&tag) {
                    d.tags.push(tag);
                }
            });
        }
        new_tag.set(String::new());
    };

    // Custom chips: saved custom tags plus anything on this entry that isn't a preset.
    let custom = Memo::new(move |_| {
        let mut list = state.custom_tags.get();
        draft.with(|d| {
            for t in &d.tags {
                if !tags::is_preset(t) && !list.contains(t) {
                    list.push(t.clone());
                }
            }
        });
        list
    });

    view! {
        <div class="space-y-3">
            <div class="flex flex-wrap gap-2">
                {PRESETS
                    .iter()
                    .map(|p| {
                        let name = p.name;
                        view! {
                            <button
                                type="button"
                                class=move || format!("{CHIP} {}", if is_on(name) { CHIP_ON } else { CHIP_OFF })
                                title=p.description
                                aria-pressed=move || is_on(name).to_string()
                                on:click=move |_| toggle(name.to_owned())
                                on:mouseenter=move |_| hovered.set(Some(name))
                                on:mouseleave=move |_| hovered.set(None)
                            >
                                {name}
                            </button>
                        }
                    })
                    .collect_view()}
                <For each=move || custom.get() key=|t| t.clone() let:tag>
                    {
                        let tag = StoredValue::new(tag);
                        let saved = move || tag.with_value(|t| state.custom_tags.with(|s| s.contains(t)));
                        view! {
                            <span class=move || {
                                format!("{CHIP} {}", if tag.with_value(|t| is_on(t)) { CHIP_ON } else { CHIP_OFF })
                            }>
                                <button
                                    type="button"
                                    aria-pressed=move || tag.with_value(|t| is_on(t)).to_string()
                                    on:click=move |_| toggle(tag.get_value())
                                >
                                    "#"{tag.get_value()}
                                </button>
                                <Show when=saved>
                                    <button
                                        type="button"
                                        class="opacity-60 hover:opacity-100"
                                        title="Remove from saved tags"
                                        aria-label="Remove from saved tags"
                                        on:click=move |_| tag.with_value(|t| state.forget_custom_tag(t))
                                    >
                                        <Icon name=IconName::X class="w-3 h-3" />
                                    </button>
                                </Show>
                            </span>
                        }
                    }
                </For>
            </div>
            <p class="min-h-4 text-[11px] text-zinc-500 dark:text-dim-400">
                {move || hovered.get().and_then(tags::preset_info).map(|p| format!("{}: {}", p.name, p.hint))}
            </p>
            <div class="flex gap-2">
                <input
                    type="text"
                    class="flex-1 min-w-0 rounded-xl border border-zinc-200 dark:border-dim-700 bg-white dark:bg-dim-950 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-sakura-300 dark:focus:ring-sakura-800"
                    placeholder="Add your own tag, e.g. Ghibli"
                    prop:value=move || new_tag.get()
                    on:input=move |ev| new_tag.set(event_target_value(&ev))
                    on:keydown=move |ev: KeyboardEvent| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            add_new();
                        }
                    }
                />
                <button
                    type="button"
                    class="inline-flex items-center gap-1 px-3 py-2 rounded-xl text-sm font-medium bg-zinc-100 dark:bg-dim-800 text-zinc-700 dark:text-dim-200 hover:bg-zinc-200 dark:hover:bg-dim-700"
                    on:click=move |_| add_new()
                >
                    <Icon name=IconName::Plus />
                    "Add"
                </button>
            </div>
        </div>
    }
}
