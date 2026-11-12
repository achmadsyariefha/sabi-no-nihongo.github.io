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
cargo test -p sabi-core -p sabi-web
cargo clippy -p sabi-core -p sabi-web --target wasm32-unknown-unknown -- -D warnings
```

Server tests need an empty Postgres database (they skip without one):

```bash
SABI_TEST_DATABASE_URL=postgres://user:pass@localhost:5432/sabi_test cargo test -p sabi-server
```

## Deploy

Pushing to `main` runs `.github/workflows/pages.yml`, which tests, builds with `trunk build --release` and publishes `dist/` to GitHub Pages.
In the repo settings, set **Pages → Source** to **GitHub Actions** once.

The site is served at <https://achmadsyariefha.github.io/sabi-no-nihongo.github.io/>. CI picks the base path from the repo name, so it also works at the domain root if the repo is moved to an account named `sabi-no-nihongo`.

To try a subpath build locally: `trunk build --public-url /sabi-no-nihongo.github.io/` (in Git Bash, prefix with `MSYS_NO_PATHCONV=1`).

## Sync server (optional)

The app works fully offline. `crates/server` is an optional Axum + Postgres server that lets several devices share one journal: open **Backup & sync** (the ⤓ button), enter the server address, and sign in or create an account.

Run it locally:

```bash
cp crates/server/.env.example .env   # then edit it
export $(grep -v '^#' .env | xargs)
cargo run -p sabi-server
```

| Variable | Meaning |
|---|---|
| `DATABASE_URL` | Postgres connection string. Tables are created on start. |
| `JWT_SECRET` | At least 32 random characters, e.g. `openssl rand -hex 32`. |
| `PORT` | Defaults to `5000`. |
| `ALLOWED_ORIGINS` | Sites allowed to call the server, comma-separated. Defaults to the GitHub Pages site and `http://localhost:8080`. |

Docker: `docker build -f crates/server/Dockerfile -t sabi-server .` (from the repo root). The image listens on `PORT` (8080 by default) and works on Fly.io, Render or Cloud Run with a hosted Postgres such as Neon or Supabase.

### Free hosting: Vercel + Neon

No credit card needed. The server runs on Vercel's Container Runtime (Hobby plan) from [`Dockerfile.vercel`](Dockerfile.vercel); the database is a free Neon Postgres.

1. **Neon** (database): create a project (region: Singapore), click **Connect**, turn **connection pooling off**, and copy the connection string (`postgresql://…neon.tech/neondb?sslmode=require`).
2. **Vercel** (server): **Add New → Project**, import this repo, keep the root directory, and add these environment variables before deploying:
   `DATABASE_URL` (the Neon string), `JWT_SECRET` (`openssl rand -hex 32`), `ALLOWED_ORIGINS=https://achmadsyariefha.github.io`, `PORT=8080`.
3. Open `https://<project>.vercel.app/api/health`; it should say `{"status":"ok"}`.

Vercel scales the server to zero after 5 idle minutes; the next request starts it again in a few seconds. [`vercel.json`](vercel.json) skips builds when only the web app changed.

The API matches the old Express backend under `/api` (`/auth`, `/entries`, `/stats`, `/review`, `/srs`), plus `POST /api/sync`, which the web app uses.

## Layout

- `crates/core` — pure-Rust logic (dates, streaks, review, SM-2, import/export, sync rules); shared by the web app and the server
- `crates/web` — Leptos UI, browser services (storage, clock, theme, sync)
- `crates/server` — optional sync server (Axum, SQLx, Postgres)
