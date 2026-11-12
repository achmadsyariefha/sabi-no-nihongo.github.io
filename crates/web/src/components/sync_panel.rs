//! "Sync across devices": sign in to a server, or see sync status.

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::icon::{Icon, IconName};
use crate::state::use_app_state;
use crate::sync::SyncStatus;

const INPUT: &str = "w-full rounded-xl border border-zinc-200 dark:border-dim-700 bg-white dark:bg-dim-950 px-3 py-2 text-sm text-zinc-900 dark:text-dim-100 placeholder:text-zinc-400 dark:placeholder:text-dim-600 focus:outline-none focus:ring-2 focus:ring-sakura-300 dark:focus:ring-sakura-800";
const LABEL: &str = "block text-xs font-semibold text-zinc-600 dark:text-dim-300 mb-1";
const BTN: &str = "px-3 py-1.5 rounded-lg text-xs font-semibold disabled:opacity-50";

fn clock_time(ms: i64) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms as f64));
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

fn host(server: &str) -> &str {
    server.split("://").nth(1).unwrap_or(server)
}

#[component]
pub fn SyncPanel() -> impl IntoView {
    let state = use_app_state();
    let s = state.sync;
    let server = RwSignal::new(s.server.get_untracked());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let go = move |create: bool| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = state
                .sign_in(&server.get_untracked(), &email.get_untracked(), &password.get_untracked(), create)
                .await;
            busy.set(false);
            match result {
                Ok(()) => password.set(String::new()),
                Err(msg) => error.set(Some(msg)),
            }
        });
    };
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        go(false);
    };

    let signed_out = move || {
        view! {
            <form class="space-y-3" on:submit=on_submit>
                <p class="text-[11px] text-zinc-500 dark:text-dim-400">
                    "Optional. Sign in to a Sabi no Nihongo server to keep your words in step across devices. "
                    "Words in this browser are added to your account; nothing is deleted."
                </p>
                <div>
                    <label for="sync-server" class=LABEL>"Server address"</label>
                    <input
                        id="sync-server"
                        type="text"
                        inputmode="url"
                        autocomplete="url"
                        class=INPUT
                        placeholder="https://your-server.example.com"
                        prop:value=move || server.get()
                        on:input=move |ev| server.set(event_target_value(&ev))
                    />
                </div>
                <div class="grid sm:grid-cols-2 gap-3">
                    <div>
                        <label for="sync-email" class=LABEL>"Email"</label>
                        <input
                            id="sync-email"
                            type="email"
                            autocomplete="username"
                            class=INPUT
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label for="sync-password" class=LABEL>"Password"</label>
                        <input
                            id="sync-password"
                            type="password"
                            autocomplete="current-password"
                            class=INPUT
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                        />
                    </div>
                </div>
                <Show when=move || busy.get()>
                    <p class="text-[11px] text-zinc-500 dark:text-dim-400" role="status">
                        "A server that was asleep can take up to a minute to wake up."
                    </p>
                </Show>
                <div class="flex flex-wrap items-center justify-end gap-2">
                    <p class="mr-auto text-xs text-sakura-600 dark:text-sakura-400" role="alert">{move || error.get()}</p>
                    <button
                        type="button"
                        class=format!("{BTN} border border-zinc-300 dark:border-dim-700 text-zinc-700 dark:text-dim-200 hover:bg-zinc-50 dark:hover:bg-dim-800")
                        disabled=move || busy.get()
                        on:click=move |_| go(true)
                    >
                        "Create account"
                    </button>
                    <button
                        type="submit"
                        class=format!("{BTN} bg-zinc-900 dark:bg-white text-white dark:text-dim-900 hover:opacity-90")
                        disabled=move || busy.get()
                    >
                        {move || if busy.get() { "Signing in…" } else { "Sign in" }}
                    </button>
                </div>
            </form>
        }
    };

    let status_line = move || match s.status.get() {
        SyncStatus::Idle => "Not synced yet.".to_owned(),
        SyncStatus::Syncing => "Syncing… (up to a minute if the server was asleep)".to_owned(),
        SyncStatus::Synced(ms) => format!("Synced at {}.", clock_time(ms)),
        SyncStatus::Failed(msg) => msg,
    };
    let failed = move || matches!(s.status.get(), SyncStatus::Failed(_));

    view! {
        {move || match s.account.get() {
            None => {
                view! {
                    {move || {
                        failed()
                            .then(|| view! { <p class="text-xs text-sakura-600 dark:text-sakura-400">{status_line}</p> })
                    }}
                    {signed_out}
                }
                    .into_any()
            }
            Some(account) => {
                view! {
                    <div class="rounded-xl border border-zinc-200 dark:border-dim-700 p-3 space-y-2">
                        <p class="text-sm text-zinc-700 dark:text-dim-200">
                            "Signed in as " <b>{account.email.clone()}</b> " on "
                            <span class="font-mono text-xs">{host(&account.server).to_owned()}</span>
                        </p>
                        <p
                            class=move || {
                                if failed() {
                                    "text-xs text-sakura-600 dark:text-sakura-400"
                                } else {
                                    "text-xs text-zinc-500 dark:text-dim-400"
                                }
                            }
                            role="status"
                        >
                            {status_line}
                        </p>
                        <div class="flex justify-end gap-2">
                            <button
                                type="button"
                                class=format!("{BTN} text-zinc-600 dark:text-dim-300 hover:bg-zinc-100 dark:hover:bg-dim-800")
                                on:click=move |_| state.sign_out()
                            >
                                "Sign out"
                            </button>
                            <button
                                type="button"
                                class=format!("{BTN} flex items-center gap-1 bg-zinc-900 dark:bg-white text-white dark:text-dim-900 hover:opacity-90")
                                disabled=move || s.status.get() == SyncStatus::Syncing
                                on:click=move |_| state.sync_now()
                            >
                                <Icon name=IconName::RotateCw class="w-3.5 h-3.5" />
                                "Sync now"
                            </button>
                        </div>
                    </div>
                }
                    .into_any()
            }
        }}
    }
}
