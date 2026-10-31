-- dbSym agentic work log (SQLite)
-- Intended for a local file under <project>/.dbsym/agent.sqlite (never commit it).
-- Schema only — no sample rows.
--
-- Schema version: 1 (see symworx_dbsym::AGENTIC_SCHEMA_VERSION)
-- Profile is catalog_meta key 'profile' = 'agent'.
-- Prompt text is not stored. body stays null in v1; body_sha256 may be set.
-- Not the study catalog (schema.sqlite.sql) and not LoadSym.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS catalog_meta (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS queries (
    id           TEXT PRIMARY KEY,
    parent_id    TEXT REFERENCES queries(id),
    harness      TEXT,
    role         TEXT,
    agent        TEXT,
    title        TEXT NOT NULL,
    body         TEXT,
    body_sha256  TEXT,
    status       TEXT NOT NULL DEFAULT 'open'
                 CHECK (status IN ('open', 'done', 'abandoned')),
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_queries_parent ON queries(parent_id);
CREATE INDEX IF NOT EXISTS idx_queries_status ON queries(status);

CREATE TABLE IF NOT EXISTS attempts (
    id                 TEXT PRIMARY KEY,
    query_id           TEXT NOT NULL REFERENCES queries(id),
    parent_attempt_id  TEXT REFERENCES attempts(id),
    agent              TEXT NOT NULL,
    kind               TEXT NOT NULL DEFAULT 'session'
                       CHECK (kind IN ('session', 'spawn', 'revise')),
    outcome            TEXT NOT NULL DEFAULT 'in_progress'
                       CHECK (outcome IN (
                           'in_progress', 'succeeded', 'failed',
                           'superseded', 'refused'
                       )),
    summary            TEXT,
    started_at         TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at        TEXT
);

CREATE INDEX IF NOT EXISTS idx_attempts_query ON attempts(query_id);
CREATE INDEX IF NOT EXISTS idx_attempts_parent ON attempts(parent_attempt_id);

CREATE TABLE IF NOT EXISTS artifacts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    attempt_id  TEXT NOT NULL REFERENCES attempts(id),
    path        TEXT NOT NULL,
    role        TEXT NOT NULL CHECK (role IN ('input', 'output', 'log')),
    sha256      TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_artifacts_attempt ON artifacts(attempt_id);
