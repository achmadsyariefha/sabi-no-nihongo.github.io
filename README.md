# Sabi no Nihongo · 一日一語

A daily Japanese word journal, written in Rust (Leptos + WebAssembly) and hosted on GitHub Pages.
Design: [docs/DESIGN.md](docs/DESIGN.md).

## Run locally

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve --open
```

## Test

```bash
cargo test -p sabi-core
cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings
```

## Deploy

Pushing to `main` runs `.github/workflows/pages.yml`, which tests, builds with `trunk build --release` and publishes `dist/` to GitHub Pages.
In the repo settings, set **Pages → Source** to **GitHub Actions** once.

## Layout

- `crates/core` — pure-Rust logic (dates, streaks, review, import/export); no browser code
- `crates/web` — Leptos UI, browser services (storage, clock, theme)
