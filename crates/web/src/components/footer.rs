use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="hidden md:block border-t border-zinc-200/80 dark:border-dim-800 py-6 text-center text-xs text-zinc-400">
            <p class="font-jp">"一日一語 (Sabi no Nihongo) — Daily Japanese Word & Translation Journal"</p>
            <p class="text-[11px] text-zinc-500 mt-1">
                "Built with Rust + Leptos • Your words stay in this browser"
            </p>
        </footer>
    }
}
