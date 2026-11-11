use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::icon::{Icon, IconName};
use crate::services::base::app_href;
use crate::state::use_app_state;

struct Tab {
    href: &'static str,
    label: &'static str,
    short: &'static str,
    icon: IconName,
}

const TABS: [Tab; 4] = [
    Tab { href: "/", label: "Today's Word", short: "Today", icon: IconName::PenLine },
    Tab { href: "/vault", label: "Word Vault", short: "Vault", icon: IconName::BookOpen },
    Tab { href: "/calendar", label: "Calendar", short: "Calendar", icon: IconName::Calendar },
    Tab { href: "/review", label: "Review", short: "Review", icon: IconName::Layers },
];

#[component]
pub fn Navbar() -> impl IntoView {
    let state = use_app_state();

    view! {
        <header class="sticky top-0 z-40 bg-[#fcfbf9]/90 dark:bg-dim-950/90 backdrop-blur-md border-b border-zinc-200/80 dark:border-dim-800 transition-colors">
            <div class="max-w-6xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between gap-2">
                <A href=app_href("/") attr:class="flex items-center gap-3 group select-none">
                    <div class="w-10 h-10 rounded-xl bg-gradient-to-tr from-sakura-500 to-sakura-400 text-white flex items-center justify-center font-jp-serif font-bold text-xl shadow-md shadow-sakura-500/20 group-hover:scale-105 transition-transform">
                        "語"
                    </div>
                    <div>
                        <div class="flex items-center gap-1.5">
                            <span class="font-bold text-lg text-zinc-900 dark:text-white tracking-tight">
                                "Sabi no Nihongo"
                            </span>
                        </div>
                        <p class="text-[11px] text-zinc-500 dark:text-dim-400 font-medium font-jp">
                            "一日一語 · one Japanese word a day"
                        </p>
                    </div>
                </A>

                <nav class="hidden md:flex items-center gap-1 bg-paper dark:bg-dim-900/90 p-1 rounded-full border border-zinc-200/80 dark:border-dim-800">
                    {TABS
                        .iter()
                        .map(|tab| {
                            view! {
                                <A
                                    href=app_href(tab.href)
                                    exact=true
                                    attr:class="flex items-center gap-2 px-3.5 py-1.5 rounded-full text-sm font-medium transition-all text-zinc-600 dark:text-dim-400 hover:text-zinc-900 dark:hover:text-dim-200 aria-[current=page]:bg-zinc-900 aria-[current=page]:text-white aria-[current=page]:font-semibold aria-[current=page]:shadow-sm dark:aria-[current=page]:bg-white dark:aria-[current=page]:text-dim-900"
                                >
                                    <Icon name=tab.icon />
                                    <span>{tab.label}</span>
                                </A>
                            }
                        })
                        .collect_view()}
                </nav>

                <div class="flex items-center gap-1.5">
                    <div
                        class="flex items-center gap-1 px-2.5 py-1.5 rounded-full bg-matcha-50 dark:bg-matcha-900/40 text-matcha-700 dark:text-matcha-300 text-sm font-semibold"
                        title="Current streak"
                    >
                        <Icon name=IconName::Flame />
                        <span>"0"</span>
                    </div>
                    <button
                        type="button"
                        class="p-2 rounded-full text-zinc-600 dark:text-dim-300 hover:bg-zinc-100 dark:hover:bg-dim-800 transition-colors"
                        title=move || if state.is_dark.get() { "Switch to light mode" } else { "Switch to dark mode" }
                        on:click=move |_| state.toggle_theme()
                    >
                        {move || {
                            if state.is_dark.get() {
                                view! { <Icon name=IconName::Sun class="w-5 h-5" /> }
                            } else {
                                view! { <Icon name=IconName::Moon class="w-5 h-5" /> }
                            }
                        }}
                    </button>
                    <button
                        type="button"
                        class="p-2 rounded-full text-zinc-400 dark:text-dim-600 cursor-not-allowed"
                        title="Backup & export (coming in Phase 3)"
                        disabled=true
                    >
                        <Icon name=IconName::Download class="w-5 h-5" />
                    </button>
                </div>
            </div>
        </header>
    }
}

/// Tab bar fixed to the bottom of the screen on phones.
#[component]
pub fn BottomNav() -> impl IntoView {
    view! {
        <nav class="md:hidden fixed bottom-0 inset-x-0 z-40 bg-[#fcfbf9]/95 dark:bg-dim-950/95 backdrop-blur-md border-t border-zinc-200/80 dark:border-dim-800 grid grid-cols-4">
            {TABS
                .iter()
                .map(|tab| {
                    view! {
                        <A
                            href=app_href(tab.href)
                            exact=true
                            attr:class="flex flex-col items-center gap-0.5 py-2.5 text-[11px] font-medium text-zinc-500 dark:text-dim-400 aria-[current=page]:text-sakura-600 dark:aria-[current=page]:text-sakura-400"
                        >
                            <Icon name=tab.icon class="w-5 h-5" />
                            <span>{tab.short}</span>
                        </A>
                    }
                })
                .collect_view()}
        </nav>
    }
}
