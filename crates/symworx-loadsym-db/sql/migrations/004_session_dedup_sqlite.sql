-- Migration 004: multi-source session linking + load dedup flags (SQLite)
-- Applied by symload catalog migrate when schema_migrations.version < 4.
-- Application code also uses ALTER TABLE guards; this file documents target DDL.
--
-- See: crate README.md, docs/loadsym-personal-starter.md (schema v4)
-- Runtime: symworx-loadsym::catalog::migrate_catalog / recompute_daily_for_date
--   (daily_loads uses counts_for_load = 1 only)

PRAGMA foreign_keys = ON;

-- Provenance / linking columns on activities (ADD COLUMN is idempotent only if guarded in app).
-- ingest_pipeline, external_id, session_group_id, counts_for_load, is_primary, match_reason

CREATE TABLE IF NOT EXISTS session_groups (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    primary_activity_id  INTEGER,
    match_method         TEXT,
    created_at           TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_activities_session_group ON activities(session_group_id);
CREATE INDEX IF NOT EXISTS idx_activities_start ON activities(ride_date, start_time);
CREATE INDEX IF NOT EXISTS idx_activities_counts ON activities(ride_date, counts_for_load);
