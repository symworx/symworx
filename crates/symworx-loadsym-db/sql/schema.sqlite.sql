-- LoadSym personal ride catalog (SQLite)
-- Intended for a local file under $VELOFIT_HOME/db/ (never commit the data file).
-- Schema only — no sample rows, no personal identifiers.
--
-- Dialect notes vs Postgres schema.sql:
--   INTEGER PRIMARY KEY AUTOINCREMENT instead of BIGSERIAL/SERIAL
--   REAL instead of DOUBLE PRECISION
--   TEXT for timestamps (ISO-8601 / datetime('now'))
--   tags stored as JSON text (e.g. '["race"]') instead of TEXT[]

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS activities (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_file     TEXT UNIQUE NOT NULL,           -- stable key relative to archive root when possible
    file_hash       TEXT,                           -- sha256 hex for content dedup
    ride_date       TEXT NOT NULL,                  -- YYYY-MM-DD
    start_time      TEXT,                           -- ISO-8601 optional
    duration_s      REAL NOT NULL,
    sport           TEXT,

    manufacturer    TEXT,
    product         TEXT,
    source_platform TEXT,

    avg_power_w     REAL,
    max_power_w     REAL,
    np_w            REAL,
    tss             REAL,
    intensity_factor REAL,
    ftp_used_w      REAL,                       -- FTP applied when scoring this ride
    ftp_history_id  INTEGER,                    -- which ftp_history row was used (if any)

    total_work_kj   REAL,

    avg_hr_bpm      REAL,
    max_hr_bpm      REAL,
    avg_cadence     REAL,
    max_cadence     REAL,
    avg_speed_kmh   REAL,
    max_speed_kmh   REAL,
    distance_m      REAL,
    elevation_gain_m REAL,

    workout_type    TEXT,
    tags            TEXT,                           -- JSON array of strings, optional

    file_size       INTEGER,
    imported_at     TEXT DEFAULT (datetime('now')),
    notes           TEXT
);

CREATE INDEX IF NOT EXISTS idx_activities_date ON activities(ride_date);
CREATE INDEX IF NOT EXISTS idx_activities_tss ON activities(tss);
CREATE INDEX IF NOT EXISTS idx_activities_hash ON activities(file_hash);

CREATE TABLE IF NOT EXISTS daily_loads (
    ride_date       TEXT PRIMARY KEY,               -- YYYY-MM-DD
    total_tss       REAL DEFAULT 0,
    total_duration_s REAL DEFAULT 0,
    ride_count      INTEGER DEFAULT 0,
    primary_sport   TEXT,
    updated_at      TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS load_metrics (
    ride_date       TEXT PRIMARY KEY,

    acute_load      REAL,
    chronic_load    REAL,
    acwr            REAL,
    risk_level      TEXT,

    ctl             REAL,
    atl             REAL,
    tsb             REAL,

    monotony        REAL,
    strain          REAL,
    computed_at     TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS power_bests (
    activity_id     INTEGER NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    duration_s      INTEGER NOT NULL,
    best_power_w    REAL NOT NULL,
    PRIMARY KEY (activity_id, duration_s)
);

CREATE INDEX IF NOT EXISTS idx_power_bests_duration ON power_bests(duration_s);

CREATE TABLE IF NOT EXISTS zones (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    effective_from  TEXT NOT NULL,
    sport           TEXT NOT NULL DEFAULT 'cycling',
    zone_type       TEXT NOT NULL,
    zone_number     INTEGER NOT NULL,
    lower           REAL,
    upper           REAL,
    name            TEXT,
    UNIQUE (effective_from, sport, zone_type, zone_number)
);

-- FTP / threshold history: time-varying values used when scoring (and re-scoring) rides.
-- Validity rule: for a given sport, a row is active from effective_from (inclusive)
-- until the next later effective_from (exclusive). Optional effective_to can document
-- an explicit end without requiring denser rows.
--
-- Lookup (ride_date D, sport S):
--   SELECT * FROM ftp_history
--   WHERE sport = S AND effective_from <= D
--     AND (effective_to IS NULL OR effective_to > D)
--   ORDER BY effective_from DESC LIMIT 1;
CREATE TABLE IF NOT EXISTS ftp_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    effective_from  TEXT NOT NULL,              -- YYYY-MM-DD when this FTP becomes valid
    effective_to    TEXT,                       -- optional exclusive end date (YYYY-MM-DD)
    ftp_w           REAL NOT NULL CHECK (ftp_w > 0),
    sport           TEXT NOT NULL DEFAULT 'cycling',
    -- How the value was obtained: ramp_test | 20min_test | map_test | estimate | manual | device
    source          TEXT,
    notes           TEXT,
    created_at      TEXT DEFAULT (datetime('now')),
    UNIQUE (sport, effective_from)
);

CREATE INDEX IF NOT EXISTS idx_ftp_history_lookup
    ON ftp_history (sport, effective_from);

-- FK from activities → ftp_history (added after both tables exist)
-- SQLite cannot ADD CONSTRAINT easily; enforce in application + index for joins.
CREATE INDEX IF NOT EXISTS idx_activities_ftp_history
    ON activities (ftp_history_id);

CREATE TABLE IF NOT EXISTS athlete (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    dob TEXT,
    gender TEXT,
    height_m REAL,
    current_weight_kg REAL,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS daily_context (
    ride_date       TEXT PRIMARY KEY,
    sleep_quality   INTEGER,
    stress          INTEGER,
    soreness        INTEGER,
    motivation      INTEGER,
    rpe             INTEGER,
    weight_kg       REAL,
    resting_hr      INTEGER,
    hrv             REAL,
    notes           TEXT
);

CREATE TABLE IF NOT EXISTS planned_workouts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    planned_date    TEXT NOT NULL,
    sport           TEXT,
    target_tss      REAL,
    target_duration_s REAL,
    description     TEXT,
    completed_activity_id INTEGER REFERENCES activities(id),
    created_at      TEXT DEFAULT (datetime('now'))
);

DROP VIEW IF EXISTS recent_load;
CREATE VIEW recent_load AS
SELECT
    d.ride_date,
    d.total_tss,
    lm.acwr,
    lm.risk_level,
    lm.ctl,
    lm.atl,
    lm.tsb,
    a.np_w,
    a.tss AS ride_tss
FROM daily_loads d
LEFT JOIN load_metrics lm USING (ride_date)
LEFT JOIN activities a ON a.ride_date = d.ride_date
ORDER BY d.ride_date DESC;
