//! Add / edit form for a journal entry.

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use sabi_core::{DailyEntry, EntryDraft};

use crate::components::icon::{Icon, IconName};
use crate::components::tag_picker::TagPicker;
use crate::state::use_app_state;

const INPUT: &str = "w-full rounded-xl border border-zinc-200 dark:border-dim-700 bg-white dark:bg-dim-950 px-4 py-2.5 text-zinc-900 dark:text-dim-100 placeholder:text-zinc-400 dark:placeholder:text-dim-600 focus:outline-none focus:ring-2 focus:ring-sakura-300 dark:focus:ring-sakura-800";
const LABEL: &str = "block text-sm font-semibold text-zinc-800 dark:text-dim-200 mb-1.5";

#[derive(Clone, Copy)]
enum Field {
    Japanese,
    Reading,
    Romaji,
    Translation,
    Notes,
    ExampleJapanese,
    ExampleReading,
    ExampleTranslation,
}

fn field_value(d: &EntryDraft, f: Field) -> String {
    let s = match f {
        Field::Japanese => &d.japanese,
        Field::Reading => &d.reading,
        Field::Romaji => &d.romaji,
        Field::Translation => &d.translation,
        Field::Notes => &d.notes,
        Field::ExampleJapanese => &d.example_japanese,
        Field::ExampleReading => &d.example_reading,
        Field::ExampleTranslation => &d.example_translation,
    };
    s.clone()
}

fn field(d: &mut EntryDraft, f: Field) -> &mut String {
    match f {
        Field::Japanese => &mut d.japanese,
        Field::Reading => &mut d.reading,
        Field::Romaji => &mut d.romaji,
        Field::Translation => &mut d.translation,
        Field::Notes => &mut d.notes,
        Field::ExampleJapanese => &mut d.example_japanese,
        Field::ExampleReading => &mut d.example_reading,
        Field::ExampleTranslation => &mut d.example_translation,
    }
}

#[component]
fn TextField(
    draft: RwSignal<EntryDraft>,
    field_id: Field,
    #[prop(into)] label: String,
    #[prop(into)] placeholder: String,
    #[prop(optional)] required: bool,
    #[prop(optional)] multiline: bool,
    #[prop(default = "")] extra_class: &'static str,
) -> impl IntoView {
    let slug: String = label
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let id = format!("f-{slug}");
    let value = move || draft.with(|d| field_value(d, field_id));
    let set = move |v: String| draft.update(|d| *field(d, field_id) = v);
    let class = format!("{INPUT} {extra_class}");
    let input = if multiline {
        view! {
            <textarea
                id=id.clone()
                rows="3"
                class=class
                placeholder=placeholder
                prop:value=value
                on:input=move |ev| set(event_target_value(&ev))
            ></textarea>
        }
        .into_any()
    } else {
        view! {
            <input
                id=id.clone()
                type="text"
                class=class
                placeholder=placeholder
                required=required
                prop:value=value
                on:input=move |ev| set(event_target_value(&ev))
            />
        }
        .into_any()
    };
    view! {
        <div>
            <label for=id class=LABEL>
                {label}
                {required.then(|| view! { <span class="text-sakura-500">" *"</span> })}
            </label>
            {input}
        </div>
    }
}

#[component]
pub fn EntryForm(
    /// The entry being edited, or `None` for a new word.
    #[prop(optional)]
    original: Option<DailyEntry>,
    #[prop(into)] badge: String,
    #[prop(into)] title: String,
    /// Called with the saved entry.
    #[prop(into)]
    on_saved: Callback<DailyEntry>,
    /// Shows a Cancel button when set.
    #[prop(default = None)]
    on_cancel: Option<Callback<()>>,
) -> impl IntoView {
    let state = use_app_state();
    let original_id = original.as_ref().map(|e| e.id.clone());
    let past_date = original.as_ref().filter(|e| e.date != state.today.get_untracked()).map(|e| e.date.format_ja());
    let draft = RwSignal::new(original.as_ref().map(EntryDraft::from_entry).unwrap_or_default());
    let show_example = RwSignal::new(draft.with_untracked(|d| !d.example_japanese.is_empty()));
    let error = RwSignal::new(None::<&'static str>);

    let submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let d = draft.get_untracked();
        match state.save_draft(&d, original_id.as_deref()) {
            Ok(saved) => {
                error.set(None);
                on_saved.run(saved);
            }
            Err(e) => error.set(Some(e.message())),
        }
    };

    view! {
        <form
            on:submit=submit
            class="rounded-3xl border border-zinc-200/80 dark:border-dim-800 bg-white dark:bg-dim-900 p-5 sm:p-8 space-y-6"
        >
            <div class="flex items-start justify-between gap-3 pb-4 border-b border-zinc-100 dark:border-dim-800">
                <div>
                    <span class="px-2.5 py-0.5 rounded-md bg-sakura-100 dark:bg-sakura-950/60 text-sakura-700 dark:text-sakura-300 text-[11px] font-bold uppercase">
                        {badge}
                    </span>
                    <h2 class="mt-2 text-lg font-bold text-zinc-900 dark:text-white">{title}</h2>
                    {past_date
                        .map(|d| {
                            view! {
                                <p class="mt-1 text-xs text-zinc-500 dark:text-dim-400">
                                    "This word is from " <span class="font-jp">{d}</span> ". Its date won't change."
                                </p>
                            }
                        })}
                </div>
                {on_cancel
                    .map(|cancel| {
                        view! {
                            <button
                                type="button"
                                class="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-700 dark:hover:text-dim-200"
                                title="Cancel"
                                aria-label="Cancel"
                                on:click=move |_| cancel.run(())
                            >
                                <Icon name=IconName::X class="w-5 h-5" />
                            </button>
                        }
                    })}
            </div>

            <TextField
                draft
                field_id=Field::Japanese
                label="Japanese word or sentence"
                placeholder="木漏れ日"
                required=true
                extra_class="font-jp text-2xl"
            />
            <div class="grid sm:grid-cols-2 gap-4">
                <TextField draft field_id=Field::Reading label="Reading (kana)" placeholder="こもれび" extra_class="font-jp" />
                <TextField draft field_id=Field::Romaji label="Romaji" placeholder="komorebi" />
            </div>
            <TextField
                draft
                field_id=Field::Translation
                label="Translation"
                placeholder="Sunlight filtering through the leaves of trees"
                required=true
            />
            <TextField
                draft
                field_id=Field::Notes
                label="Notes"
                placeholder="Where you found it, nuance, why you love it…"
                multiline=true
            />

            <div>
                <p class=LABEL>"Tags"</p>
                <TagPicker draft />
            </div>

            <label class="flex items-center gap-3 cursor-pointer select-none">
                <input
                    type="checkbox"
                    class="w-4 h-4 accent-sakura-500"
                    prop:checked=move || draft.with(|d| d.is_favorite)
                    on:change=move |ev| draft.update(|d| d.is_favorite = event_target_checked(&ev))
                />
                <span class="text-sm text-zinc-700 dark:text-dim-200">"Star as a favorite"</span>
            </label>

            <div class="rounded-2xl bg-paper dark:bg-dim-800/40 p-4 space-y-4">
                <button
                    type="button"
                    class="text-sm font-semibold text-indigoPaper-600 dark:text-indigoPaper-300 hover:underline"
                    aria-expanded=move || show_example.get().to_string()
                    on:click=move |_| show_example.update(|s| *s = !*s)
                >
                    {move || if show_example.get() { "− Hide example sentence" } else { "+ Add an example sentence" }}
                </button>
                <Show when=move || show_example.get()>
                    <TextField
                        draft
                        field_id=Field::ExampleJapanese
                        label="Example (Japanese)"
                        placeholder="森の中で木漏れ日を浴びながら散歩した。"
                        extra_class="font-jp"
                    />
                    <TextField
                        draft
                        field_id=Field::ExampleReading
                        label="Example reading"
                        placeholder="もり の なか で こもれび を…"
                        extra_class="font-jp"
                    />
                    <TextField
                        draft
                        field_id=Field::ExampleTranslation
                        label="Example translation"
                        placeholder="I took a walk in the forest, bathed in komorebi."
                    />
                </Show>
            </div>

            <div class="flex flex-col-reverse sm:flex-row sm:items-center sm:justify-end gap-3 pt-2">
                <p class="sm:mr-auto text-sm text-sakura-600 dark:text-sakura-400" role="alert">
                    {move || error.get()}
                </p>
                {on_cancel
                    .map(|cancel| {
                        view! {
                            <button
                                type="button"
                                class="px-5 py-2.5 rounded-full text-sm font-semibold text-zinc-600 dark:text-dim-300 hover:bg-zinc-100 dark:hover:bg-dim-800"
                                on:click=move |_| cancel.run(())
                            >
                                "Cancel"
                            </button>
                        }
                    })}
                <button
                    type="submit"
                    class="px-6 py-2.5 rounded-full text-sm font-semibold bg-zinc-900 dark:bg-white text-white dark:text-dim-900 hover:opacity-90"
                >
                    "Save word"
                </button>
            </div>
        </form>
    }
}
