-- Migration 002: expanded ftp_history + activities.ftp_history_id (SQLite)
-- Applied by symload catalog migrate when schema_migrations.version < 2.
-- Safe to re-run (guards on table shape / columns).

PRAGMA foreign_keys = OFF;

-- Rebuild ftp_history if it is still the v1 shape (effective_from PRIMARY KEY only).
-- Detect v1: no "id" column.
-- Application code performs the rebuild; this file documents the target DDL.

CREATE TABLE IF NOT EXISTS ftp_history_v2 (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    effective_from  TEXT NOT NULL,
    effective_to    TEXT,
    ftp_w           REAL NOT NULL CHECK (ftp_w > 0),
    sport           TEXT NOT NULL DEFAULT 'cycling',
    source          TEXT,
    notes           TEXT,
    created_at      TEXT DEFAULT (datetime('now')),
    UNIQUE (sport, effective_from)
);

-- Data copy (v1 → v2) is done in application if ftp_history exists without id.

CREATE INDEX IF NOT EXISTS idx_ftp_history_lookup
    ON ftp_history (sport, effective_from);

CREATE INDEX IF NOT EXISTS idx_activities_ftp_history
    ON activities (ftp_history_id);

PRAGMA foreign_keys = ON;
