-- LoadSym Personal Ride Database Schema (PostgreSQL recommended, SQLite compatible with small changes)
-- Focused on power meter rides (SRM, Garmin, Polar .fit) + periodization
--
-- Informed by:
--   * TrainingPeaks / WKO: TSS, CTL (42d EWMA), ATL (7d), TSB/Form, planned workouts, daily metrics (weight, HRV, stress, sleep)
--   * GoldenCheetah: detailed power curves / MMP (mean maximal power) for many durations, best efforts, athlete metrics, zones
--   * Intervals.icu and similar: eFTP history, power curve percentiles, energy system views, wellness
--   * General (TRIMP, sRPE, ACWR from symworx-loadsym)
--
-- This file lives inside the `symworx-loadsym-db` crate so it can be embedded via `include_str!`.
-- Apply against a personal database outside this repository (e.g. $VELOFIT_HOME/db/ for SQLite — see schema.sqlite.sql).

-- Schema version for migrations
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE activities (
    id              BIGSERIAL PRIMARY KEY,
    source_file     TEXT UNIQUE NOT NULL,           -- original path or stable key (for dedup)
    file_hash       TEXT,                           -- sha256 or similar for content dedup
    ride_date       DATE NOT NULL,
    start_time      TIMESTAMPTZ,
    duration_s      DOUBLE PRECISION NOT NULL,
    sport           TEXT,                           -- cycling, running, etc.

    -- Device / source (TrainingPeaks, GoldenCheetah, Strava etc. import info)
    manufacturer    TEXT,
    product         TEXT,
    source_platform TEXT,                           -- e.g. "srm-pc8", "garmin", "intervals.icu"

    -- Core load (TSS family - TrainingPeaks style)
    avg_power_w     DOUBLE PRECISION,
    max_power_w     DOUBLE PRECISION,
    np_w            DOUBLE PRECISION,               -- Normalized Power (GoldenCheetah/TP)
    tss             DOUBLE PRECISION,               -- Training Stress Score (core metric)
    intensity_factor DOUBLE PRECISION,
    ftp_used_w      DOUBLE PRECISION,             -- FTP applied when scoring this ride
    ftp_history_id  BIGINT,                       -- FK → ftp_history.id when resolved from history

    total_work_kj   DOUBLE PRECISION,

    -- HR & other
    avg_hr_bpm      DOUBLE PRECISION,
    max_hr_bpm      DOUBLE PRECISION,
    avg_cadence     DOUBLE PRECISION,
    max_cadence     DOUBLE PRECISION,
    avg_speed_kmh   DOUBLE PRECISION,
    max_speed_kmh   DOUBLE PRECISION,
    distance_m      DOUBLE PRECISION,
    elevation_gain_m DOUBLE PRECISION,

    -- Classification (useful for periodization)
    workout_type    TEXT,                           -- endurance, threshold, vo2, sprint, race, recovery...
    tags            TEXT[],                         -- flexible labels

    -- Metadata
    file_size       BIGINT,
    imported_at     TIMESTAMPTZ DEFAULT now(),
    notes           TEXT
);

CREATE INDEX idx_activities_date ON activities(ride_date);
CREATE INDEX idx_activities_tss ON activities(tss);

-- Daily rollups (one row per day, even if multiple rides)
CREATE TABLE daily_loads (
    ride_date       DATE PRIMARY KEY,
    total_tss       DOUBLE PRECISION DEFAULT 0,
    total_duration_s DOUBLE PRECISION DEFAULT 0,
    ride_count      INTEGER DEFAULT 0,
    primary_sport   TEXT,
    updated_at      TIMESTAMPTZ DEFAULT now()
);

-- Computed load metrics snapshots.
-- Supports both ACWR (our current) and PMC-style (TrainingPeaks: CTL/ATL/TSB using EWMA).
-- GoldenCheetah-style power curve tracking is in power_bests.
CREATE TABLE load_metrics (
    ride_date       DATE PRIMARY KEY,

    -- ACWR family (symworx-loadsym)
    acute_load      DOUBLE PRECISION,
    chronic_load    DOUBLE PRECISION,
    acwr            DOUBLE PRECISION,
    risk_level      TEXT,

    -- PMC family (TrainingPeaks inspired - recommend EWMA 7d/42d)
    ctl             DOUBLE PRECISION,   -- Chronic Training Load (fitness)
    atl             DOUBLE PRECISION,   -- Acute Training Load (fatigue)
    tsb             DOUBLE PRECISION,   -- Training Stress Balance = CTL - ATL (form)

    monotony        DOUBLE PRECISION,
    strain          DOUBLE PRECISION,
    computed_at     TIMESTAMPTZ DEFAULT now()
);

-- Mean Maximal Power / best efforts (core GoldenCheetah strength + Intervals.icu power curve)
-- One row per (activity, duration) for standard durations.
CREATE TABLE power_bests (
    activity_id     BIGINT REFERENCES activities(id) ON DELETE CASCADE,
    duration_s      INTEGER NOT NULL,           -- e.g. 5, 60, 300, 1200, 2400, 3600...
    best_power_w    DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (activity_id, duration_s)
);

CREATE INDEX idx_power_bests_duration ON power_bests(duration_s);

-- Power / HR zones over time (athlete configurable, like TrainingPeaks & GoldenCheetah)
CREATE TABLE zones (
    id              BIGSERIAL PRIMARY KEY,
    effective_from  DATE NOT NULL,
    sport           TEXT NOT NULL DEFAULT 'cycling',
    zone_type       TEXT NOT NULL,              -- 'power', 'hr'
    zone_number     SMALLINT NOT NULL,
    lower           DOUBLE PRECISION,
    upper           DOUBLE PRECISION,
    name            TEXT,
    UNIQUE (effective_from, sport, zone_type, zone_number)
);

-- FTP / threshold history: time-varying values used when scoring (and re-scoring) rides.
-- Validity: for a sport, row is active from effective_from (inclusive) until the next
-- later effective_from (exclusive). Optional effective_to is an explicit exclusive end.
--
-- Lookup (ride_date D, sport S):
--   SELECT * FROM ftp_history
--   WHERE sport = S AND effective_from <= D
--     AND (effective_to IS NULL OR effective_to > D)
--   ORDER BY effective_from DESC LIMIT 1;
CREATE TABLE IF NOT EXISTS ftp_history (
    id              BIGSERIAL PRIMARY KEY,
    effective_from  DATE NOT NULL,
    effective_to    DATE,
    ftp_w           DOUBLE PRECISION NOT NULL CHECK (ftp_w > 0),
    sport           TEXT NOT NULL DEFAULT 'cycling',
    source          TEXT,                         -- ramp_test | 20min_test | map_test | estimate | manual | device
    notes           TEXT,
    created_at      TIMESTAMPTZ DEFAULT now(),
    UNIQUE (sport, effective_from)
);

CREATE INDEX IF NOT EXISTS idx_ftp_history_lookup ON ftp_history (sport, effective_from);

ALTER TABLE activities
    DROP CONSTRAINT IF EXISTS activities_ftp_history_id_fkey;
ALTER TABLE activities
    ADD CONSTRAINT activities_ftp_history_id_fkey
    FOREIGN KEY (ftp_history_id) REFERENCES ftp_history(id) ON DELETE SET NULL;

-- Athlete profile / settings (single user for personal use)
CREATE TABLE athlete (
    id SERIAL PRIMARY KEY,
    name TEXT,
    dob DATE,
    gender TEXT,
    height_m DOUBLE PRECISION,
    current_weight_kg DOUBLE PRECISION,
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- Subjective + wellness data (TrainingPeaks API style + loadsym readiness)
CREATE TABLE daily_context (
    ride_date       DATE PRIMARY KEY,
    sleep_quality   SMALLINT,
    stress          SMALLINT,
    soreness        SMALLINT,
    motivation      SMALLINT,
    rpe             SMALLINT,           -- session RPE (very useful for sRPE load)
    weight_kg       DOUBLE PRECISION,
    resting_hr      SMALLINT,
    hrv             DOUBLE PRECISION,
    notes           TEXT
);

-- Planned workouts (TrainingPeaks calendar style)
CREATE TABLE planned_workouts (
    id              BIGSERIAL PRIMARY KEY,
    planned_date    DATE NOT NULL,
    sport           TEXT,
    target_tss      DOUBLE PRECISION,
    target_duration_s DOUBLE PRECISION,
    description     TEXT,
    completed_activity_id BIGINT REFERENCES activities(id),
    created_at      TIMESTAMPTZ DEFAULT now()
);

-- Example views
CREATE OR REPLACE VIEW recent_load AS
SELECT
    d.ride_date,
    d.total_tss,
    lm.acwr,
    lm.risk_level,
    lm.ctl,
    lm.atl,
    lm.tsb,
    a.np_w,
    a.tss as ride_tss
FROM daily_loads d
LEFT JOIN load_metrics lm USING (ride_date)
LEFT JOIN activities a ON a.ride_date = d.ride_date
ORDER BY d.ride_date DESC;
