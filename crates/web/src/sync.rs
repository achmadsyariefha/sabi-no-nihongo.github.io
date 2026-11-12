//! Optional sync with a Sabi no Nihongo server.
//!
//! Signed out (the default), nothing here runs and the app is local-only.
//! Signed in, changes are sent about two seconds after you make them, and the
//! app also checks for changes from other devices on start, when the window
//! gets focus, when the network comes back, and every five minutes.

use std::time::Duration;

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use sabi_core::sync::{self, SyncResponse, SyncState};
use serde::{Deserialize, Serialize};

use crate::services::api::{self, ApiError};
use crate::services::{clock, storage};
use crate::state::AppState;

const ACCOUNT_KEY: &str = "sabi.account.v1";
const SYNC_KEY: &str = "sabi.sync.v1";
const SERVER_KEY: &str = "sabi.syncServer.v1";
const DEBOUNCE: Duration = Duration::from_millis(2000);
const POLL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub server: String,
    pub email: String,
    pub token: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SyncStatus {
    Idle,
    Syncing,
    /// Last successful sync, in ms.
    Synced(i64),
    Failed(String),
}

#[derive(Clone, Copy)]
pub struct SyncSignals {
    pub account: RwSignal<Option<Account>>,
    /// Last server address used, to prefill the sign-in form.
    pub server: RwSignal<String>,
    pub status: RwSignal<SyncStatus>,
    state: RwSignal<SyncState>,
    running: RwSignal<bool>,
    again: RwSignal<bool>,
    generation: StoredValue<u64>,
}

impl SyncSignals {
    pub fn load() -> Self {
        let account = RwSignal::new(storage::load::<Option<Account>>(ACCOUNT_KEY));
        let state = RwSignal::new(storage::load::<SyncState>(SYNC_KEY));
        let saved_server = storage::load::<String>(SERVER_KEY);
        let server = RwSignal::new(if saved_server.is_empty() {
            option_env!("SABI_API_URL").unwrap_or_default().to_owned()
        } else {
            saved_server
        });
        Effect::new(move |_| account.with(|a| storage::save(ACCOUNT_KEY, a)));
        Effect::new(move |_| state.with(|s| storage::save(SYNC_KEY, s)));
        Effect::new(move |_| server.with(|s| storage::save(SERVER_KEY, s)));
        Self {
            account,
            server,
            status: RwSignal::new(SyncStatus::Idle),
            state,
            running: RwSignal::new(false),
            again: RwSignal::new(false),
            generation: StoredValue::new(0),
        }
    }
}

#[derive(Deserialize)]
struct AuthReply {
    token: String,
    user: AuthUser,
}

#[derive(Deserialize)]
struct AuthUser {
    email: String,
}

impl AppState {
    /// Starts automatic syncing. Call once at startup.
    pub fn start_sync(self) {
        let s = self.sync;
        // Send local changes shortly after they happen (debounced).
        Effect::new(move |_| {
            self.entries.track();
            self.srs.track();
            s.account.track();
            let ticket = s.generation.get_value() + 1;
            s.generation.set_value(ticket);
            set_timeout(
                move || {
                    if s.generation.get_value() == ticket && self.has_unsent_changes() {
                        self.sync_now();
                    }
                },
                DEBOUNCE,
            );
        });
        // Pull changes from other devices.
        self.sync_now();
        set_interval(move || self.sync_now(), POLL);
        let focus = window_event_listener(ev::focus, move |_| self.sync_now());
        let online = window_event_listener(ev::online, move |_| self.sync_now());
        // The app lives as long as the page; keep the listeners.
        std::mem::forget(focus);
        std::mem::forget(online);
    }

    fn has_unsent_changes(self) -> bool {
        if self.sync.account.with_untracked(Option::is_none) {
            return false;
        }
        let req = self.entries.with_untracked(|list| {
            self.srs.with_untracked(|srs| self.sync.state.with_untracked(|st| sync::outgoing(list, srs, st, 0)))
        });
        !(req.entries.is_empty() && req.deleted.is_empty() && req.srs.is_empty())
    }

    /// Sends local changes and pulls remote ones. Does nothing when signed out.
    pub fn sync_now(self) {
        let s = self.sync;
        let Some(account) = s.account.get_untracked() else { return };
        if s.running.get_untracked() {
            s.again.set(true);
            return;
        }
        s.running.set(true);
        s.status.set(SyncStatus::Syncing);
        let req = self.entries.with_untracked(|list| {
            self.srs.with_untracked(|srs| s.state.with_untracked(|st| sync::outgoing(list, srs, st, clock::now_ms())))
        });

        spawn_local(async move {
            let result: Result<SyncResponse, ApiError> =
                api::call(&account.server, "POST", "/sync", Some(&account.token), Some(&req)).await;
            match result {
                Ok(res) => {
                    let mut list = self.entries.get_untracked();
                    let mut srs = self.srs.get_untracked();
                    let before = (list.clone(), srs.clone());
                    let next = s.state.with_untracked(|st| sync::apply(&mut list, &mut srs, st, res));
                    if list != before.0 {
                        self.entries.set(list);
                    }
                    if srs != before.1 {
                        self.srs.set(srs);
                    }
                    s.state.set(next);
                    s.status.set(SyncStatus::Synced(clock::now_ms()));
                }
                Err(e) if e.is_unauthorized() => {
                    s.account.set(None);
                    s.status.set(SyncStatus::Failed("Your sign-in has expired. Please sign in again.".into()));
                }
                Err(e) => s.status.set(SyncStatus::Failed(e.message)),
            }
            s.running.set(false);
            if s.again.get_untracked() {
                s.again.set(false);
                self.sync_now();
            }
        });
    }

    /// Signs in (or creates an account) and starts syncing. On the first sync
    /// the words in this browser are added to the account.
    pub async fn sign_in(self, server: &str, email: &str, password: &str, create: bool) -> Result<(), String> {
        let server = api::normalize_server(server).ok_or("Enter the server address.")?;
        let path = if create { "/auth/register" } else { "/auth/login" };
        let body = serde_json::json!({ "email": email.trim(), "password": password });
        let reply: AuthReply = api::call(&server, "POST", path, None, Some(&body)).await.map_err(|e| e.message)?;
        self.sync.server.set(server.clone());
        self.sync.state.set(SyncState::default());
        self.sync.account.set(Some(Account { server, email: reply.user.email, token: reply.token }));
        self.sync_now();
        Ok(())
    }

    /// Stops syncing. Words stay in this browser and on the server.
    pub fn sign_out(self) {
        self.sync.account.set(None);
        self.sync.state.set(SyncState::default());
        self.sync.status.set(SyncStatus::Idle);
    }
}
