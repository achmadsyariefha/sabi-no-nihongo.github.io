//! User settings persisted under `sabi.settings.v1`.

use serde::{Deserialize, Serialize};

use crate::model::DailyLimit;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the operating system's light/dark preference.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: Theme,
    /// Words-per-day goal (not a hard limit).
    pub daily_limit: DailyLimit,
}

impl Theme {
    /// Whether dark mode is on, given what the OS prefers.
    pub fn is_dark(self, system_prefers_dark: bool) -> bool {
        match self {
            Theme::System => system_prefers_dark,
            Theme::Light => false,
            Theme::Dark => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.theme, Theme::System);
        assert_eq!(s.daily_limit, DailyLimit::One);
        // Phase 0 settings (theme only) still load.
        let old: Settings = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
        assert_eq!(old.theme, Theme::Light);
    }

    #[test]
    fn theme_serializes_lowercase() {
        let s = Settings { theme: Theme::Dark, daily_limit: DailyLimit::Three };
        assert_eq!(serde_json::to_string(&s).unwrap(), r#"{"theme":"dark","dailyLimit":3}"#);
    }

    #[test]
    fn system_theme_follows_os() {
        assert!(Theme::System.is_dark(true));
        assert!(!Theme::System.is_dark(false));
        assert!(Theme::Dark.is_dark(false));
    }
}
