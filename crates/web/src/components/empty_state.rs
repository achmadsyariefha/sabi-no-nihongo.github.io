use leptos::prelude::*;

/// Centered placeholder card for screens with nothing to show yet.
#[component]
pub fn EmptyState(
    kanji: &'static str,
    title: &'static str,
    message: &'static str,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <section class="max-w-xl mx-auto mt-6 sm:mt-12 rounded-3xl border border-zinc-200/80 dark:border-dim-800 bg-paper dark:bg-dim-900 px-6 py-12 text-center">
            <p class="font-jp-serif text-6xl sm:text-7xl font-bold text-zinc-900 dark:text-white">{kanji}</p>
            <h1 class="mt-6 text-xl font-semibold text-zinc-900 dark:text-dim-100">{title}</h1>
            <p class="mt-2 text-sm text-zinc-500 dark:text-dim-400">{message}</p>
            {children.map(|c| view! { <div class="mt-6">{c()}</div> })}
        </section>
    }
}
