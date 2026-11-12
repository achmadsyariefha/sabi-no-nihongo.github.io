-- Schema ported from the Express backend's Prisma model.
--
-- Differences:
-- * Word ids come from the browser, so they are unique per user, not globally.
-- * The example sentence is three nullable columns instead of its own table.
-- * Deleted words stay as tombstones (deleted = true) so other devices learn
--   about the deletion when they sync.
-- * Times that the browser sets (createdAt, updatedAt, dueAt, ...) are
--   milliseconds since the Unix epoch, exactly as in the JSON.
-- * change_seq increases on every write; sync uses it as a cursor.

CREATE SEQUENCE change_seq;

CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name          TEXT,
    -- Daily goal: 0 means no goal. A goal, never a limit.
    daily_limit   INT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE daily_entries (
    user_id             UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    id                  TEXT NOT NULL,
    date                TEXT NOT NULL,
    japanese            TEXT NOT NULL,
    reading             TEXT NOT NULL DEFAULT '',
    romaji              TEXT,
    translation         TEXT NOT NULL DEFAULT '',
    notes               TEXT,
    tags                TEXT[] NOT NULL DEFAULT '{}',
    is_favorite         BOOLEAN NOT NULL DEFAULT false,
    example_japanese    TEXT,
    example_reading     TEXT,
    example_translation TEXT,
    created_at          BIGINT NOT NULL,
    updated_at          BIGINT NOT NULL,
    deleted             BOOLEAN NOT NULL DEFAULT false,
    change_seq          BIGINT NOT NULL DEFAULT nextval('change_seq'),
    PRIMARY KEY (user_id, id)
);
CREATE INDEX daily_entries_user_date ON daily_entries (user_id, date);
CREATE INDEX daily_entries_user_seq ON daily_entries (user_id, change_seq);

-- Daily "Known / Need review" session.
CREATE TABLE review_sessions (
    user_id         UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    date_key        TEXT NOT NULL,
    filter_mode     TEXT NOT NULL,
    current_index   INT NOT NULL DEFAULT 0,
    known_ids       TEXT[] NOT NULL DEFAULT '{}',
    need_review_ids TEXT[] NOT NULL DEFAULT '{}',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, date_key, filter_mode)
);

-- SM-2 schedule per word. A word with no row is new and due now.
CREATE TABLE card_reviews (
    user_id          UUID NOT NULL,
    entry_id         TEXT NOT NULL,
    interval         INT NOT NULL DEFAULT 0,
    ease             DOUBLE PRECISION NOT NULL DEFAULT 2.5,
    repetitions      INT NOT NULL DEFAULT 0,
    lapses           INT NOT NULL DEFAULT 0,
    due_at           BIGINT NOT NULL DEFAULT 0,
    last_reviewed_at BIGINT,
    change_seq       BIGINT NOT NULL DEFAULT nextval('change_seq'),
    PRIMARY KEY (user_id, entry_id),
    FOREIGN KEY (user_id, entry_id) REFERENCES daily_entries (user_id, id) ON DELETE CASCADE
);
CREATE INDEX card_reviews_user_due ON card_reviews (user_id, due_at);
CREATE INDEX card_reviews_user_seq ON card_reviews (user_id, change_seq);

-- One row per answered SM-2 card.
CREATE TABLE review_logs (
    id              BIGSERIAL PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    entry_id        TEXT NOT NULL,
    rating          TEXT NOT NULL,
    interval_before INT NOT NULL,
    interval_after  INT NOT NULL,
    reviewed_at     BIGINT NOT NULL
);
CREATE INDEX review_logs_user_time ON review_logs (user_id, reviewed_at);
