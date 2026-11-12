//! Backup window: export, import and reset.

use leptos::ev;
use leptos::html::{Button, Input};
use leptos::prelude::*;
use leptos::task::spawn_local;
use sabi_core::DailyEntry;
use sabi_core::{export, import};

use crate::components::icon::{Icon, IconName};
use crate::services::{clock, files};
use crate::state::use_app_state;

const EXPORT_BTN: &str = "p-3.5 rounded-2xl bg-zinc-50 dark:bg-dim-800/80 border border-zinc-200/80 dark:border-dim-700 hover:border-sakura-400 text-left transition-all disabled:opacity-40 disabled:hover:border-zinc-200/80";
const SECTION_LABEL: &str = "text-xs font-bold uppercase tracking-wider text-zinc-500";

#[derive(Clone, Copy)]
enum Format {
    Json,
    Csv,
    Anki,
}

fn download(format: Format, entries: &[DailyEntry]) {
    let day = clock::today();
    let (name, mime, body) = match format {
        Format::Json => (format!("sabi-no-nihongo-backup-{day}.json"), "application/json", export::to_json(entries)),
        Format::Csv => (format!("sabi-no-nihongo-words-{day}.csv"), "text/csv;charset=utf-8", export::to_csv(entries)),
        Format::Anki => {
            (format!("sabi-no-nihongo-anki-{day}.txt"), "text/tab-separated-values;charset=utf-8", export::to_anki_tsv(entries))
        }
    };
    if let Err(err) = files::download(&name, mime, &body) {
        leptos::logging::warn!("download failed: {err:?}");
    }
}

#[derive(Clone, PartialEq)]
enum Step {
    Idle,
    Reading,
    Preview { entries: Vec<DailyEntry>, skipped: usize, added: usize, replaced: usize },
    Imported(usize),
    Failed(String),
    ConfirmReset,
    WasReset,
}

fn words(n: usize) -> String {
    if n == 1 { "1 word".into() } else { format!("{n} words") }
}

#[component]
pub fn BackupModal() -> impl IntoView {
    let state = use_app_state();
    view! {
        <Show when=move || state.backup_open.get()>
            <BackupDialog />
        </Show>
    }
}

#[component]
fn BackupDialog() -> impl IntoView {
    let state = use_app_state();
    let step = RwSignal::new(Step::Idle);
    let close = move || state.backup_open.set(false);
    let count = move || state.entries.with(Vec::len);
    let empty = move || count() == 0;

    // Escape closes; focus starts on the close button.
    let handle = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            close();
        }
    });
    on_cleanup(move || handle.remove());
    let close_btn = NodeRef::<Button>::new();
    Effect::new(move |_| {
        if let Some(b) = close_btn.get() {
            let _ = b.focus();
        }
    });

    let export = move |f: Format| state.entries.with_untracked(|list| download(f, list));

    let picker = NodeRef::<Input>::new();
    let on_pick = move |_| {
        let Some(input) = picker.get() else { return };
        let Some(file) = files::take_picked_file(&input) else { return };
        step.set(Step::Reading);
        spawn_local(async move {
            let next = match files::read_text(&file).await {
                Err(_) => Step::Failed("Couldn't read that file.".into()),
                Ok(text) => match import::parse(&text, clock::now_ms()) {
                    Err(e) => Step::Failed(e.message().into()),
                    Ok(got) if got.entries.is_empty() => Step::Failed(if got.skipped > 0 {
                        format!("No usable words in this file ({} skipped).", got.skipped)
                    } else {
                        "This file has no words in it.".into()
                    }),
                    Ok(got) => {
                        let plan = state.entries.with_untracked(|list| import::plan_merge(list, &got.entries));
                        Step::Preview { entries: got.entries, skipped: got.skipped, added: plan.added, replaced: plan.replaced }
                    }
                },
            };
            step.set(next);
        });
    };
    let confirm_import = move |_| {
        if let Step::Preview { entries, .. } = step.get_untracked() {
            let n = entries.len();
            state.import(entries);
            step.set(Step::Imported(n));
        }
    };

    let status = move || match step.get() {
        Step::Idle | Step::ConfirmReset | Step::WasReset => None,
        Step::Reading => Some(view! { <p class="text-xs text-zinc-500">"Reading the file…"</p> }.into_any()),
        Step::Preview { entries, skipped, added, replaced } => {
            let mut detail = format!("{} new", added);
            if replaced > 0 {
                detail.push_str(&format!(", {replaced} will replace words with the same id"));
            }
            let skipped_note = (skipped > 0).then(|| {
                format!(" {} skipped (missing an id, the Japanese text, or a valid date).", words(skipped))
            });
            Some(
                view! {
                    <div class="rounded-xl border border-sakura-200 dark:border-sakura-900/60 bg-sakura-50/60 dark:bg-sakura-950/30 p-3 space-y-3">
                        <p class="text-sm text-zinc-700 dark:text-dim-200">
                            "Found " <b>{words(entries.len())}</b> " (" {detail} ")." {skipped_note}
                        </p>
                        <div class="flex justify-end gap-2">
                            <button
                                type="button"
                                class="px-3 py-1.5 rounded-lg text-xs font-semibold text-zinc-600 dark:text-dim-300 hover:bg-zinc-100 dark:hover:bg-dim-800"
                                on:click=move |_| step.set(Step::Idle)
                            >
                                "Cancel"
                            </button>
                            <button
                                type="button"
                                class="px-3 py-1.5 rounded-lg text-xs font-bold bg-sakura-500 hover:bg-sakura-600 text-white"
                                on:click=confirm_import
                            >
                                "Import " {words(entries.len())}
                            </button>
                        </div>
                    </div>
                }
                .into_any(),
            )
        }
        Step::Imported(n) => Some(
            view! { <p class="text-sm font-semibold text-matcha-700 dark:text-matcha-400">"Imported " {words(n)} "."</p> }
                .into_any(),
        ),
        Step::Failed(msg) => Some(view! { <p class="text-sm text-sakura-600 dark:text-sakura-400">{msg}</p> }.into_any()),
    };

    let reset_panel = move || {
        (step.get() == Step::ConfirmReset).then(|| {
            view! {
                <div class="rounded-xl border border-rose-200 dark:border-rose-900/60 bg-rose-50/70 dark:bg-rose-950/30 p-3 space-y-3">
                    <p class="text-sm text-rose-800 dark:text-rose-200">
                        "This deletes all " <b>{words(count())}</b>
                        ", your custom tags and your review history from this browser. It can't be undone."
                    </p>
                    <div class="flex flex-wrap justify-end gap-2">
                        <button
                            type="button"
                            class="px-3 py-1.5 rounded-lg text-xs font-semibold text-zinc-600 dark:text-dim-300 hover:bg-zinc-100 dark:hover:bg-dim-800"
                            on:click=move |_| step.set(Step::Idle)
                        >
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class="px-3 py-1.5 rounded-lg text-xs font-semibold border border-zinc-300 dark:border-dim-700 text-zinc-700 dark:text-dim-200 hover:bg-white dark:hover:bg-dim-800 disabled:opacity-40"
                            disabled=empty
                            on:click=move |_| export(Format::Json)
                        >
                            "Download a backup first"
                        </button>
                        <button
                            type="button"
                            class="px-3 py-1.5 rounded-lg text-xs font-bold bg-rose-600 hover:bg-rose-700 text-white"
                            on:click=move |_| {
                                state.reset_all();
                                step.set(Step::WasReset);
                            }
                        >
                            "Delete everything"
                        </button>
                    </div>
                </div>
            }
        })
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" aria-hidden="true" on:click=move |_| close()></div>
            <div
                class="relative bg-white dark:bg-dim-900 rounded-3xl border border-zinc-200 dark:border-dim-800 p-6 max-w-lg w-full max-h-[calc(100vh-2rem)] overflow-y-auto shadow-2xl space-y-6"
                role="dialog"
                aria-modal="true"
                aria-labelledby="backup-title"
            >
                <div class="flex items-center justify-between border-b border-zinc-100 dark:border-dim-800 pb-4">
                    <div class="flex items-center gap-2.5">
                        <div class="p-2 rounded-xl bg-sakura-50 dark:bg-sakura-950/60 text-sakura-600 dark:text-sakura-400">
                            <Icon name=IconName::Download class="w-5 h-5" />
                        </div>
                        <div>
                            <h2 id="backup-title" class="text-lg font-bold text-zinc-900 dark:text-white">"Backup & export"</h2>
                            <p class="text-xs text-zinc-500">"Save your words to a file, or bring them back."</p>
                        </div>
                    </div>
                    <button
                        type="button"
                        node_ref=close_btn
                        class="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-dim-200"
                        aria-label="Close"
                        on:click=move |_| close()
                    >
                        <Icon name=IconName::X class="w-5 h-5" />
                    </button>
                </div>

                <section class="space-y-3">
                    <h3 class=SECTION_LABEL>"Export your collection (" {move || words(count())} ")"</h3>
                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                        <button type="button" class=EXPORT_BTN disabled=empty on:click=move |_| export(Format::Json)>
                            <Icon name=IconName::FileJson class="w-5 h-5 text-blue-500 mb-2" />
                            <span class="block text-xs font-bold text-zinc-900 dark:text-white">"JSON backup"</span>
                            <span class="block text-[10px] text-zinc-500">"Full backup you can import"</span>
                        </button>
                        <button type="button" class=EXPORT_BTN disabled=empty on:click=move |_| export(Format::Csv)>
                            <Icon name=IconName::FileSpreadsheet class="w-5 h-5 text-matcha-500 mb-2" />
                            <span class="block text-xs font-bold text-zinc-900 dark:text-white">"CSV file"</span>
                            <span class="block text-[10px] text-zinc-500">"Excel or Google Sheets"</span>
                        </button>
                        <button type="button" class=EXPORT_BTN disabled=empty on:click=move |_| export(Format::Anki)>
                            <Icon name=IconName::Layers class="w-5 h-5 text-purple-500 mb-2" />
                            <span class="block text-xs font-bold text-zinc-900 dark:text-white">"Anki deck"</span>
                            <span class="block text-[10px] text-zinc-500">"Text file for Anki's Import"</span>
                        </button>
                    </div>
                </section>

                <section class="space-y-3 pt-2 border-t border-zinc-100 dark:border-dim-800">
                    <h3 class=SECTION_LABEL>"Import a backup"</h3>
                    <input
                        type="file"
                        accept=".json,application/json"
                        class="hidden"
                        node_ref=picker
                        on:change=on_pick
                    />
                    <button
                        type="button"
                        class="w-full py-2.5 px-4 rounded-xl border border-dashed border-zinc-300 dark:border-dim-700 hover:border-sakura-400 bg-zinc-50/50 dark:bg-dim-800/40 text-xs font-semibold text-zinc-700 dark:text-dim-300 flex items-center justify-center gap-2 transition-colors"
                        on:click=move |_| {
                            if let Some(input) = picker.get() {
                                input.click();
                            }
                        }
                    >
                        <Icon name=IconName::Upload class="w-4 h-4 text-sakura-500" />
                        "Choose a JSON backup"
                    </button>
                    <p class="text-[11px] text-zinc-400 dark:text-dim-500">
                        "Works with backups from this app and from Kotoba Daily. Words are added to your collection; nothing is deleted."
                    </p>
                    {status}
                </section>

                <section class="space-y-3 pt-2 border-t border-zinc-100 dark:border-dim-800">
                    <div class="flex items-center justify-between gap-2 text-xs">
                        <span class="flex items-center gap-1.5 text-[11px] text-zinc-500">
                            <Icon name=IconName::ShieldCheck class="w-3.5 h-3.5 text-matcha-500" />
                            "Saved only in this browser"
                        </span>
                        <button
                            type="button"
                            class="flex items-center gap-1 text-xs text-zinc-400 hover:text-rose-600 dark:hover:text-rose-400"
                            on:click=move |_| step.set(Step::ConfirmReset)
                        >
                            <Icon name=IconName::TriangleAlert class="w-3.5 h-3.5" />
                            "Delete all data…"
                        </button>
                    </div>
                    {reset_panel}
                    {move || {
                        (step.get() == Step::WasReset)
                            .then(|| {
                                view! {
                                    <p class="text-sm font-semibold text-matcha-700 dark:text-matcha-400">
                                        "All data deleted. Fresh start!"
                                    </p>
                                }
                            })
                    }}
                </section>
            </div>
        </div>
    }
}
