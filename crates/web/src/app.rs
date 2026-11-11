use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::footer::Footer;
use crate::components::navbar::{BottomNav, Navbar};
use crate::pages::{calendar::CalendarPage, not_found::NotFound, review::ReviewPage, today::TodayPage, vault::VaultPage};
use crate::state::AppState;

#[component]
pub fn App() -> impl IntoView {
    let state = AppState::load();
    state.sync_theme();
    provide_context(state);

    view! {
        <Router>
            <div class="min-h-screen flex flex-col">
                <Navbar />
                <main class="flex-1 max-w-6xl w-full mx-auto px-4 sm:px-6 py-6 sm:py-8 pb-24 sm:pb-8">
                    <Routes fallback=|| view! { <NotFound /> }>
                        <Route path=path!("/") view=TodayPage />
                        <Route path=path!("/vault") view=VaultPage />
                        <Route path=path!("/calendar") view=CalendarPage />
                        <Route path=path!("/review") view=ReviewPage />
                    </Routes>
                </main>
                <Footer />
                <BottomNav />
            </div>
        </Router>
    }
}
