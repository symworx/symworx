# Notes — dbSym catalog

Public surface: crate README. This file is the model and reuse notes.

## Why this crate

The rest of SymWorx analyzes series and tables. It does not record *whose* data
a file is, which visit it belonged to, or how two projects stay isolated. dbSym
installs a catalog **into a project directory** (or onto an edge node): shared
engine, isolated data.

Proving archive: Physiological Synchrony (`~/research/synchrony`) — coded ids,
screening vs profile visits, rest vs exercise, RR files plus cortisol/GH tables.
The crate template has no Synchrony CHECKs (`s1` / `p2e`). After init, the
**study** owns `<project>/.dbsym/schema.sqlite.sql` and may add tables there.

## Reuse (this workspace)

| Need | Existing piece |
|------|----------------|
| SQL-only schema crate shape | `symworx-loadsym-db` (`include_str!`, `SCHEMA_VERSION`, `get_schema`, `catalog_meta`) |
| SQLite init | `symworx-loadsym::catalog::init_catalog` (pragma, `execute_batch`, migrations row) |
| Domain errors | `thiserror` enums (`LoadSymError`, `EmbedError`) — **not** `SymError` on the default path (`symworx-error` always depends on `csv`) |
| `rusqlite` | same `0.32` + `bundled` as LoadSym |
| CLI | argv parsing like `symload` (no extra clap crate) |
| Units | height in meters, mass in kg (`AGENTS.md`) |
| Signal bytes | `symworx-io` after resolving `object_key` |

LoadSym **tables** stay in `symworx-loadsym-db`. Optional SQL pack later if a
node needs FIT + PPG in one file.

## Store layout

```text
<project>/
  .dbsym/
    schema.sqlite.sql   # study-owned; commit; init copies template once
    dbsym.sqlite        # gitignore
    objects/            # gitignore
    .gitignore          # *.sqlite and objects/ only
```

SQLite default. Postgres dialect later, same dual-file pattern as LoadSym.

**Study profile:** multi-subject; seed `exercise_science` (optional).

**Node profile:** same DDL; typically one `coded_id` / `sid`; skip name/DOB hashing.

## Identity

- PK: random surrogate UUID (assigned on ingest, not at `init`).
- `coded_id` first-class (`ps001`, device `sid`).
- `combined_hash` column exists for later de-id; unused in v1.

## Sessions vs files vs observations

- **sessions** — visit / capture window (`label`, `kind`, `condition`, `started_at`).
- **file_records** — RR, VO2, surveys, PDFs (`role` = `raw` \| `derived`).
- **observations** — tabular points (cortisol timepoints); `time_s` first-class; other fields EAV.

v1 ingest (next): CSV + file register. Excel stays convert-to-CSV.

## Locked choices

| Choice | Decision |
|--------|----------|
| Engine | SQLite default; Postgres later |
| Flexibility | EAV + `time_s`; files for waveforms |
| Subject id | UUID; coded ids valid without name/DOB |
| LoadSym | Separate schema this build |
| Default deps | `thiserror` only; `rusqlite` behind `sqlite` |
| Interface | Library + `symdb` CLI; no TUI key yet |
