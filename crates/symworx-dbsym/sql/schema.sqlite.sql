-- dbSym per-study / edge catalog (SQLite)
-- Intended for a local file under <project>/.dbsym/ (never commit the data file).
-- Schema only — no sample rows, no identifiers.
--
-- Schema version: 1 (see symworx_dbsym::SCHEMA_VERSION)
-- Study and node share this DDL. Profile is catalog_meta ('study' | 'node').
-- Waveforms live as files under .dbsym/objects/; this catalog stores provenance.
-- Not the LoadSym personal ride catalog (symworx-loadsym-db).
--
-- Dialect notes (Postgres later, if needed, with same pattern as loadsym-db):
--   INTEGER PRIMARY KEY AUTOINCREMENT instead of BIGSERIAL
--   REAL instead of DOUBLE PRECISION
--   TEXT for timestamps (ISO-8601 / datetime('now'))

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Pipeline / install state (profile, project name). Not subject PII.
CREATE TABLE IF NOT EXISTS catalog_meta (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ingest_batch (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    uploader TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    accepted INTEGER NOT NULL DEFAULT 0,
    duplicates INTEGER NOT NULL DEFAULT 0,
    rejected INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS subjects (
    id TEXT PRIMARY KEY,
    combined_hash TEXT UNIQUE,
    coded_id TEXT UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    ingest_batch_id INTEGER REFERENCES ingest_batch(id),
    status TEXT NOT NULL DEFAULT 'accepted',
    reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_subjects_coded_id ON subjects(coded_id);
CREATE INDEX IF NOT EXISTS idx_subjects_batch ON subjects(ingest_batch_id);

-- Visit / capture window. Labels and conditions are free text (not study enums).
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id TEXT NOT NULL REFERENCES subjects(id),
    label TEXT NOT NULL,
    kind TEXT,
    condition TEXT,
    started_at TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_subject ON sessions(subject_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);

CREATE TABLE IF NOT EXISTS file_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id TEXT REFERENCES subjects(id),
    session_id INTEGER REFERENCES sessions(id),
    ingest_batch_id INTEGER REFERENCES ingest_batch(id),
    modality TEXT,
    format TEXT,
    object_key TEXT NOT NULL,
    original_name TEXT,
    sha256 TEXT,
    role TEXT NOT NULL DEFAULT 'raw' CHECK (role IN ('raw', 'derived')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_file_records_subject ON file_records(subject_id);
CREATE INDEX IF NOT EXISTS idx_file_records_session ON file_records(session_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_file_records_object_key ON file_records(object_key);

CREATE TABLE IF NOT EXISTS attributes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL CHECK (type IN ('numeric', 'text', 'date')),
    description TEXT,
    preset TEXT
);

-- Tabular repeated measures. Waveforms are file_records.
CREATE TABLE IF NOT EXISTS observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id TEXT NOT NULL REFERENCES subjects(id),
    session_id INTEGER REFERENCES sessions(id),
    time_s REAL,
    ingest_batch_id INTEGER REFERENCES ingest_batch(id),
    source_row INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_observations_subject ON observations(subject_id);
CREATE INDEX IF NOT EXISTS idx_observations_session ON observations(session_id);

CREATE TABLE IF NOT EXISTS observation_values (
    observation_id INTEGER NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
    attribute_id INTEGER NOT NULL REFERENCES attributes(id),
    value TEXT,
    PRIMARY KEY (observation_id, attribute_id)
);

-- After `symdb init`, edit the *copy* at <project>/.dbsym/schema.sqlite.sql.
-- That file is the study source of truth; this crate template is not overwritten
-- onto an existing copy. Keep core table names if you want ingest / TUI later.
-- Add study-specific tables or columns below the copy, then: symdb apply.
