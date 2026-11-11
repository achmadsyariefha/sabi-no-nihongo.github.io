//! App-wide reactive state, provided once via context.

use leptos::prelude::*;
use sabi_core::settings::{Settings, Theme};

use crate::services::{storage, theme};

const SETTINGS_KEY: &str = "sabi.settings.v1";

#[derive(Clone, Copy)]
pub struct AppState {
    pub settings: RwSignal<Settings>,
    /// Whether dark mode is currently showing (theme setting + OS preference).
    pub is_dark: Memo<bool>,
}

impl AppState {
    pub fn load() -> Self {
        let settings = RwSignal::new(storage::load::<Settings>(SETTINGS_KEY));
        let system_dark = theme::system_prefers_dark();
        let is_dark = Memo::new(move |_| settings.with(|s| s.theme.is_dark(system_dark)));

        // Persist settings whenever they change.
        Effect::new(move |_| settings.with(|s| storage::save(SETTINGS_KEY, s)));

        Self { settings, is_dark }
    }

    /// Keeps the `dark` class on <html> in sync with the theme.
    pub fn sync_theme(self) {
        Effect::new(move |_| theme::apply_dark_class(self.is_dark.get()));
    }

    pub fn toggle_theme(self) {
        let next = if self.is_dark.get_untracked() { Theme::Light } else { Theme::Dark };
        self.settings.update(|s| s.theme = next);
    }
}

pub fn use_app_state() -> AppState {
    expect_context::<AppState>()
}
