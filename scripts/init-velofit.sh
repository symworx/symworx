#!/usr/bin/env bash
# init-velofit.sh — create a personal LoadSym archive + empty SQLite catalog.
#
# Usage (from the SymWorx workspace root, or any cwd with cargo available):
#   ./scripts/init-velofit.sh
#   VELOFIT_HOME=/path/to/archive ./scripts/init-velofit.sh
#   SYMLOAD_BIN=/path/to/symload ./scripts/init-velofit.sh
#
# Creates:
#   $VELOFIT_HOME/
#     raw/  (email/ polar/ manual/)
#     inbox/
#     db/loadsym.sqlite   (empty schema; not overwritten if already present)
#     .tmp/
#
# Does not create secrets (.env). See crates/symworx-loadsym-db/docs/loadsym-personal-starter.md

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VELOFIT_HOME="${VELOFIT_HOME:-${HOME}/velofit}"
DB_PATH="${SYMLOAD_DB:-${VELOFIT_HOME}/db/loadsym.sqlite}"

echo "VELOFIT_HOME = ${VELOFIT_HOME}"
echo "catalog      = ${DB_PATH}"

mkdir -p \
  "${VELOFIT_HOME}/raw/email" \
  "${VELOFIT_HOME}/raw/polar" \
  "${VELOFIT_HOME}/raw/manual" \
  "${VELOFIT_HOME}/inbox" \
  "${VELOFIT_HOME}/db" \
  "${VELOFIT_HOME}/.tmp"

echo "Created archive directories."

if [[ -f "${DB_PATH}" ]]; then
  echo "Catalog already exists; leaving it unchanged."
else
  export VELOFIT_HOME
  export SYMLOAD_DB="${DB_PATH}"

  resolve_symload() {
    if [[ -n "${SYMLOAD_BIN:-}" && -x "${SYMLOAD_BIN}" ]]; then
      echo "${SYMLOAD_BIN}"
      return 0
    fi
    for cand in \
      "${ROOT}/target/release/symload" \
      "${ROOT}/target/debug/symload"
    do
      if [[ -x "${cand}" ]]; then
        echo "${cand}"
        return 0
      fi
    done
    return 1
  }

  init_via_cargo() {
    if ! command -v cargo >/dev/null 2>&1; then
      return 1
    fi
    echo "Initializing empty catalog via cargo (symworx-loadsym, features=sqlite) …"
    (
      cd "${ROOT}"
      cargo run -q -p symworx-loadsym --features sqlite --bin symload -- db init --db "${DB_PATH}"
    )
  }

  if SYMLOAD="$(resolve_symload)"; then
    echo "Initializing catalog with ${SYMLOAD} …"
    if ! out="$("${SYMLOAD}" db init --db "${DB_PATH}" 2>&1)"; then
      echo "${out}" >&2
      # Binary may have been built without the sqlite feature.
      if echo "${out}" | grep -qi 'features sqlite\|requires --features'; then
        if init_via_cargo; then
          :
        else
          echo "error: ${SYMLOAD} was built without --features sqlite, and cargo is unavailable." >&2
          echo "  Rebuild: cargo build -p symworx-loadsym --features sqlite --bin symload" >&2
          echo "  Archive directories were still created under ${VELOFIT_HOME}" >&2
          exit 1
        fi
      else
        echo "error: db init failed (dirs still created under ${VELOFIT_HOME})" >&2
        exit 1
      fi
    else
      printf '%s\n' "${out}"
    fi
  elif init_via_cargo; then
    :
  else
    echo "error: no usable symload binary and cargo not on PATH." >&2
    echo "  Build once: cargo build -p symworx-loadsym --features sqlite --bin symload" >&2
    echo "  or: SYMLOAD_BIN=/path/to/symload $0" >&2
    echo "  Archive directories were still created under ${VELOFIT_HOME}" >&2
    exit 1
  fi
  echo "Catalog ready."
fi

cat <<EOF

Next steps:
  1. Drop .fit files into  ${VELOFIT_HOME}/raw/  or  ${VELOFIT_HOME}/inbox/
  2. Ingest:  cargo run -p symworx-loadsym --features sqlite -- ingest --ftp 280
  3. TUI:     cargo run -p symworx-tui --bin symview   → Home → 3 LoadSym

Optional (IMAP / Polar): see crates/symworx-loadsym-db/docs/loadsym-personal-starter.md
EOF
