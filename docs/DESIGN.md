# Sabi no Nihongo — Design

**Sabi no Nihongo — 一日一語** is a Rust rewrite of Kotoba Daily (`../japanese-daily-word`): a personal journal where you log one (or a few) Japanese words every day, keep a streak, browse your vault, and review with flashcards.

Hosted on GitHub Pages at `https://sabi-no-nihongo.github.io`, so the app must be a **static site**: Rust compiled to WebAssembly, no server required.

---

## 1. Goals

| Goal | Notes |
|---|---|
| Feature parity with the React app | Today, Vault, Calendar, Review, Backup (see §3) |
| 100% Rust app logic | Models, streaks, review, import/export written once in Rust and unit-tested natively |
| Static hosting | `trunk build --release` → `dist/` → GitHub Pages via Actions |
| Offline-first | Data lives in the browser (`localStorage`); works without network |
| Backup-compatible | Can import a JSON backup exported from the React app, and vice versa |
| Ready for a backend later | Same core crate can power an Axum server that replaces `japanese-daily-word-backend` |

**Not in v1:** accounts, cloud sync, dictionary lookup, native mobile app, spaced repetition (SM-2 comes in Phase 4).

### 1.1 Decisions

| Topic | Decision |
|---|---|
| Name | **Sabi no Nihongo**, tagline 一日一語. `<title>`: `Sabi no Nihongo · 一日一語` |
| Day start | **Local midnight** in the browser's timezone, for streaks, daily limit, review reset and review log |
| Review in v1 | **Known / Need review** session like the React app; SM-2 added later |
| First launch | **Starts empty** — no sample words; Today shows an empty state inviting the first word |
| Backend | **Axum (Rust), last phase** — local-only until then; server reuses `core` (see §12). Go was considered and rejected because it would duplicate `core` logic |

---

## 2. Tech stack

| Concern | Choice | Why |
|---|---|---|
| UI framework | **Leptos** (CSR mode) | Fine-grained signals map closely to React hooks; mature; small WASM output |
| Build / dev server | **Trunk** | Zero-config WASM bundling, hot reload, built-in Tailwind hook |
| Styling | **Tailwind CSS** (standalone CLI) | Reuse the reference palette and classes almost 1:1; scan `*.rs` files |
| Routing | `leptos_router` | `/`, `/vault`, `/calendar`, `/review` |
| Storage | `gloo-storage` (`localStorage`) | Same model as the reference; simple and synchronous |
| Serialization | `serde` + `serde_json` | Backup format identical to the React app (camelCase) |
| Dates | `time` crate + `js_sys::Date` for local offset | Pure-Rust date math, testable outside the browser |
| IDs | `uuid` (v4, `js` feature) | Same as reference |
| Text-to-speech | `web-sys` `SpeechSynthesis` | Direct port of `speakJapanese` |
| Icons | Inline SVG components (Lucide paths) | No JS icon library needed |
| Confetti | Small JS snippet via `wasm-bindgen` | Only JS in the project; optional |
| Tests | `cargo test` (core), `wasm-bindgen-test` (web, light) | Core logic is pure Rust, so most tests run natively |

Alternatives considered: **Dioxus** (good, but heavier and more opinionated about its CLI), **Yew** (older VDOM model, slower to write). Leptos is the best fit for a React-to-Rust port.

---

## 3. Features (carried over from the reference)

1. **Today** — dashboard with streak, today's count vs. daily limit (1 / 2 / 3 / 5 / unlimited), proverb of the day, and the entry form:
   - Japanese (required), reading, romaji, translation, notes
   - Tags: presets (Favorite, Anime & Manga, Poetic, Idiom (四字熟語), Daily Life, Emotion, Food & Nature, Slang & Casual, JLPT N5–N1) + saved custom tags
   - Favorite toggle, optional example sentence (JP / reading / translation)
   - 🔊 pronunciation button; prev/next when a day has several words
   - Logging is locked to today; past entries can be edited
2. **Vault** — searchable, filterable list of all words (text search, tag filter, favorites only), edit / delete.
3. **Calendar** — month grid marking logged days, pick a day to see its words; activity heatmaps for words added and cards reviewed.
4. **Review** — flashcards (all or favorites), "Known" / "Need review", progress autosaves and resets at the day boundary; each review is counted in the review log.
5. **Backup** — export JSON, CSV (UTF-8 BOM, Excel-friendly), Anki TSV; import JSON with validation; offer export before reset.
6. **Theme** — light / dark, follows system by default, remembered.

---

## 4. Architecture

```
┌──────────────────────────── browser ────────────────────────────┐
│                                                                 │
│  web crate (Leptos, WASM)                                       │
│  ┌─────────────┐   ┌──────────────┐   ┌──────────────────────┐  │
│  │   pages/    │──▶│  AppState    │──▶│  services/           │  │
│  │ components/ │   │  (signals)   │   │  storage · speech ·  │  │
│  └─────────────┘   └──────┬───────┘   │  clock · download    │  │
│                           │           └──────────┬───────────┘  │
│                           ▼                      ▼              │
│                    core crate (pure Rust)   localStorage        │
│         models · streak · review · srs · import · export        │
└─────────────────────────────────────────────────────────────────┘
                            ▲
                            │ same crate, later
                  server crate (Axum + SQLx + Postgres)
```

Rule: **`core` never touches the browser.** Anything that needs `window`, `localStorage`, speech, or the current time goes through a `services/` module in `web`. That keeps `core` fully testable with plain `cargo test`.

### 4.1 Workspace layout

```
sabi-no-nihongo.github.io/
├── Cargo.toml                 # [workspace] members = ["crates/*"]
├── Trunk.toml                 # dist dir, public_url = "/"
├── index.html                 # Trunk entry: fonts, tailwind link, wasm
├── tailwind.config.js         # copied from reference, content = ["crates/web/src/**/*.rs"]
├── style/input.css            # @tailwind base/components/utilities + custom
├── public/                    # favicon, 404.html is generated in CI
├── .github/workflows/pages.yml
└── crates/
    ├── core/                  # sabi-core: no web deps
    │   └── src/
    │       ├── lib.rs
    │       ├── model.rs       # DailyEntry, ExampleSentence, Settings, ReviewState
    │       ├── date.rs        # DateKey (YYYY-MM-DD), day boundary, JP formatting
    │       ├── streak.rs      # StreakStats::compute
    │       ├── tags.rs        # presets, normalize ("#foo" → "foo"), Idiom migration
    │       ├── review.rs      # daily flashcard session (known / need review)
    │       ├── srs.rs         # SM-2 scheduler (Phase 4, port of backend utils/srs.ts)
    │       ├── import.rs      # normalize_imported_entries → (Vec<Entry>, skipped)
    │       ├── export.rs      # to_json, to_csv, to_anki_tsv
    │       └── proverbs.rs    # DAILY_PROVERBS (no sample entries)
    └── web/                   # sabi-web: Leptos app
        └── src/
            ├── main.rs        # mount_to_body(App)
            ├── app.rs         # Router, Navbar, Footer, theme
            ├── state.rs       # AppState (signals) + persistence effects
            ├── services/
            │   ├── storage.rs # typed get/set over gloo-storage, versioned keys
            │   ├── speech.rs  # speak_japanese(text, rate)
            │   ├── clock.rs   # today() -> DateKey using local offset
            │   └── download.rs# Blob + <a download>
            ├── pages/
            │   ├── today.rs   # TodayDashboard + TodayInput
            │   ├── vault.rs
            │   ├── calendar.rs
            │   └── review.rs
            └── components/
                ├── navbar.rs  entry_card.rs  entry_form.rs  tag_picker.rs
                ├── heatmap.rs calendar_grid.rs flashcard.rs
                ├── backup_modal.rs  modal.rs  icon.rs
```

---

## 5. Data model (`core::model`)

Field names serialize to the **same camelCase JSON** as the React app, so backups move between the two.

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyEntry {
    pub id: String,
    pub date: DateKey,                 // "YYYY-MM-DD", validated newtype
    pub japanese: String,
    pub reading: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub romaji: Option<String>,
    pub translation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example_sentence: Option<ExampleSentence>,
    pub created_at: i64,               // ms since epoch
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExampleSentence {
    pub japanese: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading: Option<String>,
    pub translation: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DailyLimit { One, Two, Three, Five, Unlimited }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateKey { year: i32, month: u8, day: u8 }   // serde as "YYYY-MM-DD"

pub struct StreakStats {
    pub current: u32,
    pub longest: u32,
    pub total_entries: usize,
    pub favorites: usize,
    pub today_count: usize,
    pub completed: BTreeSet<DateKey>,
}

pub struct ReviewSession {
    pub date_key: DateKey,
    pub filter: ReviewFilter,          // All | Favorites
    pub current_index: usize,
    pub known: Vec<String>,
    pub need_review: Vec<String>,
    pub last_updated: i64,
}
```

### 5.1 Storage keys (`localStorage`)

| Key | Value |
|---|---|
| `sabi.entries.v1` | `Vec<DailyEntry>` |
| `sabi.settings.v1` | `{ dailyLimit, theme }` |
| `sabi.customTags.v1` | `Vec<String>` |
| `sabi.reviewSession.v1` | `ReviewSession` |
| `sabi.reviewLog.v1` | `BTreeMap<DateKey, u32>` |

Every value is wrapped as `{ "v": 1, "data": ... }` so later migrations can branch on `v`. On load: parse → migrate → validate (reuse `core::import`) → fall back to defaults on error, never panic. A missing `sabi.entries.v1` means a fresh install: the app starts with an empty list (no seed data).

---

## 6. Core logic

| Module | Behavior | Ported from |
|---|---|---|
| `streak` | Unique dates sorted; longest run of consecutive days; current streak counts back from today, or from yesterday if today isn't logged yet | `calculateStreakStats` |
| `date` | One `today(offset)` function used everywhere; `format_ja(DateKey)` → `2026年11月11日（水）` | `getTodayDateString`, `getUtc8DateKey`, `formatJapaneseDate` |
| `import` | Skip items missing id / japanese / valid date; dedupe tags; coerce optional fields; return skipped count | `normalizeImportedEntries` |
| `export` | JSON (pretty), CSV with BOM and `""` escaping, Anki TSV (front `日本語 [reading]`, back HTML) | `exportTo*` |
| `tags` | Strip leading `#`, trim, dedupe; migrate `Idiom` → `Idiom (四字熟語)` | `loadCustomTags` |
| `review` | Build deck (all / favorites), mark known / need review, advance, reset when day changes | `ReviewModal` + flashcard state |
| `srs` *(Phase 4)* | SM-2: pass → 1d, 6d, then `interval × ease`; fail → reset, ease −0.2 (min 1.3), due in 10 min | backend `utils/srs.ts` |

**Fix along the way:** the reference uses *local* time for logging and streaks but *UTC+8* for the flashcard reset. The Rust version uses **local midnight for everything**: `services::clock::today()` reads the browser offset via `js_sys::Date::get_timezone_offset()` and passes a `DateKey` into `core`. `core` functions take "today" as an argument and never read the clock themselves, so tests can pin any date.

---

## 7. UI state (`web::state`)

```rust
#[derive(Clone, Copy)]
pub struct AppState {
    pub entries: RwSignal<Vec<DailyEntry>>,
    pub settings: RwSignal<Settings>,
    pub custom_tags: RwSignal<Vec<String>>,
    pub review_log: RwSignal<BTreeMap<DateKey, u32>>,
    pub editing: RwSignal<Option<String>>,      // id of entry being edited
    pub stats: Memo<StreakStats>,               // derived from entries + today
}
```

- Provided once via `provide_context(AppState::load())`, read with `use_context`.
- One `Effect` per signal writes it back to `localStorage` (same as the React `useEffect` persistence).
- Actions are plain methods: `save_entry`, `delete_entry`, `toggle_favorite`, `import(entries)`, `reset_all`.

---

## 8. Screens

Visual language comes straight from the reference: warm paper background `#fcfbf9`, **sakura** accent, **matcha** for success / streaks, **zen** neutrals, **dim** greys for dark mode; Noto Serif JP for the big word, Noto Sans JP + Plus Jakarta Sans for UI.

```
┌────────────────────────────────────────────────────────────┐
│ 🌸 Sabi no Nihongo  Today Vault Calendar Review  🔥12 ☾ ⤓ │  Navbar
├────────────────────────────────────────────────────────────┤
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │
│ │ Streak  12🔥 │ │ Today  1 / 2 │ │ 継続は力なり          │ │  Dashboard
│ │ Best    30   │ │ ▓▓▓▓▓░░░░░   │ │ Continuity is power. │ │
│ └──────────────┘ └──────────────┘ └──────────────────────┘ │
│ ┌────────────────────────────────────────────────────────┐ │
│ │                     木漏れ日                    🔊  ★   │ │  Today's word
│ │                 こもれび (komorebi)                     │ │
│ │   Sunlight filtering through the leaves of trees       │ │
│ │   #Poetic #Nature              ‹ 1 / 2 ›   ✎ Edit      │ │
│ └────────────────────────────────────────────────────────┘ │
│ [ + Add another word ]   Review 8 cards due →              │
└────────────────────────────────────────────────────────────┘
```

| Screen | Main components |
|---|---|
| Today | `StatCards`, `ProverbCard`, `EntryCard` (pager), `EntryForm` + `TagPicker`, limit selector. Empty state on first launch: big 一日一語, one line ("Log your first word today"), the form open, and a link to import a backup |
| Vault | search box, tag chips, favorites toggle, grid of `EntryCard` (compact), edit / delete. Empty state: "No words yet" + link to Today |
| Calendar | `CalendarGrid` (month nav, dots on logged days), day panel, two `Heatmap`s (words, reviews) |
| Review | filter (All / ★), `Flashcard` (tap to flip, 🔊), Known / Need review, progress bar, summary + confetti. Progress resets at local midnight. Empty state when there are no words to review |
| Backup modal | Export JSON / CSV / Anki, Import JSON (shows imported / skipped), Reset with "export first" |

Mobile: navbar collapses to a bottom tab bar under `sm`; cards stack in one column; 16px side gutter.

---

## 9. Build and deploy

**Local**

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve --open
```

**`Trunk.toml`**

```toml
[build]
target = "index.html"
dist = "dist"
public_url = "/"            # user/org site → served at domain root
```

**GitHub Actions (`.github/workflows/pages.yml`)**

1. Checkout → install Rust stable + `wasm32-unknown-unknown` → cache cargo.
2. `cargo test -p sabi-core`
3. Install Trunk + Tailwind standalone CLI → `trunk build --release`
4. `cp dist/index.html dist/404.html` (so deep links like `/vault` work on Pages)
5. `actions/upload-pages-artifact` → `actions/deploy-pages`

Size target: < 400 KB gzipped WASM (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `wasm-opt -Oz` via Trunk).

---

## 10. Testing

- **core** (native `cargo test`): streaks across month/year edges and gaps; today-vs-yesterday current streak; import with malformed items; CSV escaping of quotes/commas/newlines; SM-2 intervals; tag normalization; JSON round-trip against a fixture exported from the React app.
- **web**: a few `wasm-bindgen-test` smoke tests for storage load/save and corrupted-storage fallback; manual checklist per screen in light/dark and mobile width.

---

## 11. Roadmap

| Phase | Deliverable |
|---|---|
| **0 — Skeleton** | Workspace, Trunk + Tailwind, empty Leptos app with navbar + theme, Pages deploy working |
| **1 — Log & keep** | `core::model/date/streak/tags`, storage service, Today page (form, limit, pager, TTS), Vault |
| **2 — See progress** | Calendar, heatmaps, review log |
| **3 — Review & backup** | Flashcards with daily autosave/reset, Backup modal (JSON/CSV/Anki, import, reset) |
| **4 — Spaced repetition & polish** | `core::srs` (SM-2) with a per-card schedule in `sabi.srs.v1` and a "Due today" review mode; confetti, keyboard shortcuts, PWA manifest + service worker for offline install |
| **5 — Optional sync** | `crates/server` (Axum + SQLx + Postgres) reusing `core`, porting the Express routes; web app gains an optional "sign in to sync" (see §12) |

---

## 12. Backend (Phase 5)

An optional sync server that replaces `japanese-daily-word-backend` (Express + Prisma). The web app stays fully usable without it.

| Concern | Choice |
|---|---|
| HTTP | **Axum** on Tokio, `tower-http` for CORS (allow `https://sabi-no-nihongo.github.io`), tracing, compression |
| Database | **Postgres** via **SQLx** (compile-time checked queries), migrations in `crates/server/migrations` |
| Auth | `argon2` password hashes, `jsonwebtoken` access tokens |
| Validation | `core::import` rules + `serde` types shared with the web app |
| Shared code | `sabi-core` for models, streak, review, SRS — the server is the source of truth for SRS schedules and stats when signed in |
| Hosting | Single binary in a small Docker image on Fly.io / Render / Cloud Run; Postgres on Neon or Supabase |

**Schema** is ported from the Prisma model: `users`, `daily_entries` (example sentence as nullable columns instead of a separate table), `review_sessions`, `card_reviews`, `review_logs`.

**Routes** match the Express API so the move is one-to-one:

| Route | Purpose |
|---|---|
| `POST /auth/register`, `POST /auth/login`, `GET/PATCH /auth/me` | Account, daily limit setting |
| `GET/POST /entries`, `GET/PATCH/DELETE /entries/:id` | CRUD; `GET` supports `?since=<updatedAt>` for sync |
| `GET /stats` | Streaks and counts (computed with `core::streak`) |
| `GET /review`, `POST /review/reset`, `POST /review/mark` | Daily Known / Need review session |
| `GET /srs/due`, `POST /srs/review` | SM-2 cards (`core::srs`) |

**Sync model:** the browser keeps `localStorage` as an offline cache. On sign-in it uploads local entries; afterwards it pulls changes with `?since=` and pushes edits, resolving conflicts by latest `updatedAt`.

**Workspace addition:**

```
crates/server/
├── Cargo.toml        # axum, tokio, sqlx, tower-http, argon2, jsonwebtoken, sabi-core
├── migrations/
└── src/
    ├── main.rs       # config, router, graceful shutdown
    ├── routes/       # auth.rs entries.rs stats.rs review.rs srs.rs
    ├── db.rs         # SQLx pool + queries
    ├── auth.rs       # JWT extractor
    └── error.rs      # AppError → JSON responses
```

CI: `cargo test --workspace` with a Postgres service container; deploy the server separately from GitHub Pages.

## 13. Open questions

None blocking. Decisions are recorded in §1.1.
