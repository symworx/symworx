#!/usr/bin/env bash
# Bump the shared SymWorx workspace version in lockstep.
#
# Source of truth: root Cargo.toml  [workspace.package] version
# Member crates already use version = { workspace = true } and do not need edits.
#
# Updates (only these sites):
#   1. [workspace.package] version
#   2. Every internal symworx-*  version = "…"  under [workspace.dependencies]
#      (Cargo forbids version.workspace = true there; must stay in lockstep)
#   3. Python package versions: bindings/python/pyproject*.toml
#      and bindings/python/symworx/loadsym/pyproject.toml
#   4. bindings/r path deps that pin symworx-* versions
#
# Does NOT touch:
#   - third-party dependency versions (ndarray, polars, …)
#   - historical CHANGELOG entries (optional --changelog only adds a stub section)
#   - LoadSym schema versions, demo file names, docs “v4”, etc.
#
# Usage:
#   ./scripts/bump-version.sh              # print current version + consistency check
#   ./scripts/bump-version.sh patch        # 0.1.1 → 0.1.2
#   ./scripts/bump-version.sh minor        # 0.1.1 → 0.2.0
#   ./scripts/bump-version.sh major        # 0.1.1 → 1.0.0
#   ./scripts/bump-version.sh set 0.2.0    # explicit target (up or down)
#   ./scripts/bump-version.sh set 0.2.0-rc.1
#   ./scripts/bump-version.sh set 0.1.0    # undo an accidental bump (downgrade)
#   ./scripts/bump-version.sh set 0.1.1    # re-sync drifted files to workspace version
#   ./scripts/bump-version.sh sync         # after editing Cargo.toml by hand:
#                                         # push [workspace.package] version to all other sites
#   ./scripts/bump-version.sh patch --dry-run
#   ./scripts/bump-version.sh minor --changelog
#   ./scripts/bump-version.sh set 0.1.0 --yes   # skip interactive confirm on downgrade
#
# Manual edit workflow (simplest mental model):
#   1. Set the correct number only in root Cargo.toml:
#        [workspace.package]
#        version = "0.1.2"
#      (you can ignore the internal symworx-* version= lines and pyprojects)
#   2. ./scripts/bump-version.sh sync
#      → rewrites [workspace.dependencies] symworx-* pins, Python pyprojects,
#        and R path pins to match. Member crates already use workspace = true.
#
# Going too far (downgrade / undo):
#   Edit Cargo.toml back to the lower version, then:  ./scripts/bump-version.sh sync
#   Or:  ./scripts/bump-version.sh set <previous> [--dry-run]
#   Script rewrites the same lockstep sites. It does NOT rewrite git history,
#   delete tags, or remove CHANGELOG sections. Safe only if that higher version
#   was NOT published to crates.io / PyPI yet (and ideally not tagged on the
#   remote). If already published, cut the next higher version instead.
#
# Safe workflow:
#   1. On release/vX.Y.Z (or a release-prep branch):
#        ./scripts/bump-version.sh patch --dry-run
#        ./scripts/bump-version.sh patch --changelog
#   2. Fill in CHANGELOG.md notes
#   3. git diff, commit, open PR → main
#   4. After merge: git tag -a vX.Y.Z && git push origin vX.Y.Z
#
# See DEVELOPMENT.md § Releasing.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DRY_RUN=0
DO_CHANGELOG=0
ASSUME_YES=0
ACTION=""
EXPLICIT=""

usage() {
  sed -n '2,55p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage 0 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --changelog) DO_CHANGELOG=1; shift ;;
    -y|--yes) ASSUME_YES=1; shift ;;
    show|current|check)
      ACTION="show"; shift ;;
    sync)
      ACTION="sync"; shift ;;
    patch|minor|major)
      ACTION="$1"; shift ;;
    set)
      ACTION="set"
      shift
      EXPLICIT="${1:-}"
      if [[ -z "$EXPLICIT" ]]; then
        echo "error: set requires a version, e.g. set 0.2.0" >&2
        exit 1
      fi
      shift
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage 1
      ;;
  esac
done

ACTION="${ACTION:-show}"

# ---------------------------------------------------------------------------
# Version helpers
# ---------------------------------------------------------------------------

# Read [workspace.package] version (same awk shape as release.yml).
read_workspace_version() {
  awk '
    $0 ~ /^\[workspace\.package\]/ { in_pkg=1; next }
    in_pkg && $0 ~ /^\[/ { in_pkg=0 }
    in_pkg && $0 ~ /^version/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, RSTART+1, RLENGTH-2)
        exit
      }
    }
  ' Cargo.toml
}

# SemVer-ish: major.minor.patch with optional -prerelease / +build.
is_valid_version() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]
}

# Strip pre-release / build for arithmetic; keep base X.Y.Z.
base_version() {
  local v="$1"
  v="${v%%-*}"
  v="${v%%+*}"
  echo "$v"
}

bump_semver() {
  local kind="$1" current="$2"
  local base major minor patch
  base="$(base_version "$current")"
  IFS=. read -r major minor patch <<<"$base"
  case "$kind" in
    major) echo "$((major + 1)).0.0" ;;
    minor) echo "${major}.$((minor + 1)).0" ;;
    patch) echo "${major}.${minor}.$((patch + 1))" ;;
    *) echo "error: bad bump kind: $kind" >&2; exit 1 ;;
  esac
}

# Numeric compare on X.Y.Z bases only (prerelease ignored for ordering).
# Prints: -1 if a<b, 0 if a==b, 1 if a>b
cmp_base_versions() {
  local a b
  a="$(base_version "$1")"
  b="$(base_version "$2")"
  local am an ap bm bn bp
  IFS=. read -r am an ap <<<"$a"
  IFS=. read -r bm bn bp <<<"$b"
  if (( am < bm )); then echo -1; return; fi
  if (( am > bm )); then echo 1; return; fi
  if (( an < bn )); then echo -1; return; fi
  if (( an > bn )); then echo 1; return; fi
  if (( ap < bp )); then echo -1; return; fi
  if (( ap > bp )); then echo 1; return; fi
  echo 0
}

confirm_or_abort() {
  local prompt="$1"
  if [[ "$ASSUME_YES" -eq 1 || "$DRY_RUN" -eq 1 ]]; then
    return 0
  fi
  if [[ ! -t 0 ]]; then
    echo "error: non-interactive shell; re-run with --yes to confirm" >&2
    exit 1
  fi
  local reply
  read -r -p "${prompt} [y/N] " reply
  case "$reply" in
    y|Y|yes|YES) return 0 ;;
    *) echo "Aborted."; exit 1 ;;
  esac
}

# ---------------------------------------------------------------------------
# File lists (only package / publish metadata — not third-party deps)
# ---------------------------------------------------------------------------

# Maturin / PyPI metadata (must match workspace for release publish).
python_version_files=(
  bindings/python/pyproject.toml
  bindings/python/pyproject-biosym.toml
  bindings/python/pyproject-core.toml
  bindings/python/pyproject-loadsym.toml
  bindings/python/symworx/loadsym/pyproject.toml
)

# ---------------------------------------------------------------------------
# Table helpers + version readers
# ---------------------------------------------------------------------------

# Columns: name (28) | col2 (12) | col3 (12)
table_rule() {
  printf "  %-28s  %-12s  %-12s\n" \
    "----------------------------" "------------" "------------"
}

table_hdr() {
  printf "  %-28s  %-12s  %-12s\n" "$1" "$2" "$3"
  table_rule
}

table_row() {
  printf "  %-28s  %-12s  %-12s\n" "$1" "$2" "$3"
}

# Short label for a pyproject path (basename is enough; disambiguate nested).
py_label() {
  local f="$1"
  case "$f" in
    bindings/python/symworx/loadsym/pyproject.toml) echo "loadsym/pyproject.toml" ;;
    bindings/python/*) echo "${f#bindings/python/}" ;;
    *) echo "$f" ;;
  esac
}

# Print "name\tversion" for each internal crate under [workspace.dependencies].
read_cargo_internal_versions() {
  awk '
    $0 ~ /^\[workspace\.dependencies\]/ { in_deps=1; next }
    in_deps && $0 ~ /^\[/ { in_deps=0 }
    in_deps && $0 ~ /^symworx-/ && match($0, /^symworx-[a-z0-9-]+/) {
      name = substr($0, RSTART, RLENGTH)
      if (match($0, /version = "[^"]+"/)) {
        ver = substr($0, RSTART + 11, RLENGTH - 12)
        printf "%s\t%s\n", name, ver
      }
    }
  ' Cargo.toml
}

# Print "name\tversion" for R path pins.
read_r_pin_versions() {
  local f="bindings/r/Cargo.toml"
  [[ -f "$f" ]] || return 0
  awk '
    match($0, /^[[:space:]]*symworx-[a-z0-9-]+/) {
      name = $0
      sub(/^[[:space:]]+/, "", name)
      sub(/[[:space:]].*/, "", name)
      if (match($0, /version = "[^"]+"/)) {
        ver = substr($0, RSTART + 11, RLENGTH - 12)
        printf "%s\t%s\n", name, ver
      }
    }
  ' "$f"
}

# ---------------------------------------------------------------------------
# Consistency report (read-only)
# ---------------------------------------------------------------------------

report_versions() {
  local expected="$1"
  local ok=1
  local name ver f status mism=0 total=0

  echo "Workspace package: ${expected}"
  echo

  echo "Cargo  [workspace.dependencies]  (internal crates)"
  table_hdr "crate" "version" "status"
  while IFS=$'\t' read -r name ver; do
    [[ -n "$name" ]] || continue
    total=$((total + 1))
    if [[ "$ver" == "$expected" ]]; then
      table_row "$name" "$ver" "ok"
    else
      table_row "$name" "$ver" "want ${expected}"
      ok=0
      mism=$((mism + 1))
    fi
  done < <(read_cargo_internal_versions)
  echo "  (${total} crates$([ "$mism" -gt 0 ] && echo ", ${mism} mismatch" || echo ", all match"))"
  echo

  total=0
  mism=0
  echo "Python  pyproject versions"
  table_hdr "package" "version" "status"
  for f in "${python_version_files[@]}"; do
    total=$((total + 1))
    name="$(py_label "$f")"
    if [[ ! -f "$f" ]]; then
      table_row "$name" "-" "missing"
      ok=0
      mism=$((mism + 1))
      continue
    fi
    ver="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$f")"
    if [[ "$ver" == "$expected" ]]; then
      table_row "$name" "$ver" "ok"
    else
      table_row "$name" "$ver" "want ${expected}"
      ok=0
      mism=$((mism + 1))
    fi
  done
  echo "  (${total} packages$([ "$mism" -gt 0 ] && echo ", ${mism} mismatch" || echo ", all match"))"
  echo

  if [[ -f bindings/r/Cargo.toml ]]; then
    total=0
    mism=0
    echo "R  path pins  (bindings/r/Cargo.toml)"
    table_hdr "crate" "version" "status"
    while IFS=$'\t' read -r name ver; do
      [[ -n "$name" ]] || continue
      total=$((total + 1))
      if [[ "$ver" == "$expected" ]]; then
        table_row "$name" "$ver" "ok"
      else
        table_row "$name" "$ver" "want ${expected}"
        ok=0
        mism=$((mism + 1))
      fi
    done < <(read_r_pin_versions)
    echo "  (${total} pins$([ "$mism" -gt 0 ] && echo ", ${mism} mismatch" || echo ", all match"))"
    echo
  fi

  if grep -Eq "^## \[${expected}\]" CHANGELOG.md 2>/dev/null; then
    echo "CHANGELOG.md: has ## [${expected}] section"
  else
    echo "CHANGELOG.md: no ## [${expected}] section yet (release CI requires it on release/* and tags)"
  fi

  return $((1 - ok))
}

# Preview planned rewrites as compact old → new tables (dry-run or pre-apply).
print_change_plan() {
  local old_ws="$1" new="$2"
  local name ver f cur changes=0

  echo "Plan → ${new}"
  echo

  echo "Cargo.toml"
  table_hdr "crate / site" "old" "new"
  if [[ "$old_ws" != "$new" ]]; then
    table_row "[workspace.package]" "$old_ws" "$new"
    changes=$((changes + 1))
  fi
  while IFS=$'\t' read -r name ver; do
    [[ -n "$name" ]] || continue
    if [[ "$ver" != "$new" ]]; then
      table_row "$name" "$ver" "$new"
      changes=$((changes + 1))
    fi
  done < <(read_cargo_internal_versions)
  if [[ "$changes" -eq 0 ]]; then
    table_row "(no cargo changes)" "-" "-"
  fi
  echo

  changes=0
  echo "Python"
  table_hdr "package" "old" "new"
  for f in "${python_version_files[@]}"; do
    [[ -f "$f" ]] || continue
    cur="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$f")"
    if [[ "$cur" != "$new" ]]; then
      table_row "$(py_label "$f")" "$cur" "$new"
      changes=$((changes + 1))
    fi
  done
  if [[ "$changes" -eq 0 ]]; then
    table_row "(no python changes)" "-" "-"
  fi
  echo

  if [[ -f bindings/r/Cargo.toml ]]; then
    changes=0
    echo "R  (bindings/r/Cargo.toml)"
    table_hdr "crate" "old" "new"
    while IFS=$'\t' read -r name ver; do
      [[ -n "$name" ]] || continue
      if [[ "$ver" != "$new" ]]; then
        table_row "$name" "$ver" "$new"
        changes=$((changes + 1))
      fi
    done < <(read_r_pin_versions)
    if [[ "$changes" -eq 0 ]]; then
      table_row "(no R pin changes)" "-" "-"
    fi
    echo
  fi
}

# ---------------------------------------------------------------------------
# Apply bump
# ---------------------------------------------------------------------------

# Portable in-place sed (GNU and BSD).
sed_i() {
  local expr="$1" file="$2"
  if sed --version >/dev/null 2>&1; then
    sed -i -E "$expr" "$file"
  else
    sed -i '' -E "$expr" "$file"
  fi
}

bump_cargo_toml() {
  local new="$1"
  local tmp
  tmp="$(mktemp)"
  # Section-aware rewrite so we never touch third-party dep versions.
  awk -v new="$new" '
    BEGIN { sec = "" }
    /^\[workspace\.package\]/ { sec = "pkg"; print; next }
    /^\[workspace\.dependencies\]/ { sec = "deps"; print; next }
    /^\[/ { sec = ""; print; next }

    sec == "pkg" && /^version[[:space:]]*=/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, 1, RSTART) new substr($0, RSTART + RLENGTH - 1)
        next
      }
    }

    sec == "deps" && /^symworx-/ && /version = "/ {
      # Only the version= field of internal crates (path + version tables).
      gsub(/version = "[^"]+"/, "version = \"" new "\"")
      print
      next
    }

    { print }
  ' Cargo.toml >"$tmp"
  mv "$tmp" Cargo.toml
}

bump_python_pyprojects() {
  local new="$1"
  local f
  for f in "${python_version_files[@]}"; do
    [[ -f "$f" ]] || continue
    # Replace whatever is currently on the project version line.
    sed_i "s/^(version[[:space:]]*=[[:space:]]*)\"[^\"]+\"/\1\"${new}\"/" "$f"
  done
}

bump_r_bindings() {
  local new="$1"
  local f="bindings/r/Cargo.toml"
  [[ -f "$f" ]] || return 0
  # Only lines that are path deps on workspace crates.
  if sed --version >/dev/null 2>&1; then
    sed -i -E \
      's/^([[:space:]]*symworx-[a-z0-9-]+[[:space:]]*=[[:space:]]*\{[^}]*version = ")[^"]+(")/\1'"${new}"'\2/' \
      "$f"
  else
    sed -i '' -E \
      's/^([[:space:]]*symworx-[a-z0-9-]+[[:space:]]*=[[:space:]]*\{[^}]*version = ")[^"]+(")/\1'"${new}"'\2/' \
      "$f"
  fi
}

maybe_stub_changelog() {
  local new="$1"
  if grep -Eq "^## \[${new}\]" CHANGELOG.md 2>/dev/null; then
    echo "CHANGELOG.md already has ## [${new}]"
    return 0
  fi
  local date
  date="$(date +%Y-%m-%d)"
  local stub
  stub=$(cat <<EOF
## [${new}] - ${date}

### Added

- 

### Changed

- 

### Notes

- 

EOF
)
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "--- dry-run: would prepend CHANGELOG stub for [${new}] ---"
    echo "$stub"
    return 0
  fi
  local tmp
  tmp="$(mktemp)"
  # Insert after the SemVer blurb / first blank block, before the first ## heading.
  awk -v stub="$stub" '
    BEGIN { inserted = 0 }
    /^## \[/ && !inserted {
      print stub
      inserted = 1
    }
    { print }
    END {
      if (!inserted) {
        print ""
        print stub
      }
    }
  ' CHANGELOG.md >"$tmp"
  mv "$tmp" CHANGELOG.md
  echo "CHANGELOG.md: added stub section ## [${new}] - ${date}"
  echo "  → edit the bullets, then keep the heading for release CI."
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

CURRENT="$(read_workspace_version)"
if [[ -z "$CURRENT" ]]; then
  echo "error: could not read [workspace.package] version from Cargo.toml" >&2
  exit 1
fi

if ! is_valid_version "$CURRENT"; then
  echo "error: current workspace version looks invalid: '${CURRENT}'" >&2
  exit 1
fi

if [[ "$ACTION" == "show" ]]; then
  echo "Current workspace version: ${CURRENT}"
  echo
  if report_versions "$CURRENT"; then
    echo "All lockstep sites match."
    exit 0
  else
    echo "Some sites are out of sync (fix with a bump or manual edit)."
    exit 1
  fi
fi

case "$ACTION" in
  patch|minor|major)
    NEW="$(bump_semver "$ACTION" "$CURRENT")"
    ;;
  sync)
    # Canonical number already lives in [workspace.package]; fan it out.
    NEW="$CURRENT"
    ;;
  set)
    NEW="$EXPLICIT"
    if ! is_valid_version "$NEW"; then
      echo "error: invalid version '${NEW}' (want X.Y.Z or X.Y.Z-prerelease)" >&2
      exit 1
    fi
    ;;
  *)
    echo "error: unhandled action: $ACTION" >&2
    exit 1
    ;;
esac

DIRECTION="bump"
if [[ "$NEW" == "$CURRENT" ]]; then
  DIRECTION="sync"
elif [[ "$(cmp_base_versions "$NEW" "$CURRENT")" -lt 0 ]]; then
  DIRECTION="downgrade"
elif [[ "$(cmp_base_versions "$NEW" "$CURRENT")" -eq 0 && "$NEW" != "$CURRENT" ]]; then
  # Same X.Y.Z base, different pre-release tag (e.g. 0.2.0-rc.2 → 0.2.0-rc.1)
  DIRECTION="adjust"
fi

if [[ "$DIRECTION" == "sync" ]]; then
  if report_versions "$CURRENT" >/dev/null 2>&1; then
    echo "Already at ${CURRENT}; all lockstep sites match. Nothing to do."
    report_versions "$CURRENT" || true
    exit 0
  fi
  echo "Workspace is already ${CURRENT}, but some lockstep sites drifted."
  echo "Re-syncing those sites to ${CURRENT}."
  echo
else
  case "$DIRECTION" in
    bump)      echo "Bump:      ${CURRENT} → ${NEW}" ;;
    downgrade) echo "Downgrade: ${CURRENT} → ${NEW}" ;;
    adjust)    echo "Adjust:    ${CURRENT} → ${NEW}" ;;
  esac
fi

[[ "$DRY_RUN" -eq 1 ]] && echo "(dry-run: no files will be written)"
echo

print_change_plan "$CURRENT" "$NEW"

if [[ "$DIRECTION" == "downgrade" ]]; then
  cat <<EOF
Note: this only rewrites package metadata in the working tree.
  • Does not delete git tags, CHANGELOG history, or remote releases.
  • Safe if ${CURRENT} was never published / tagged remotely.
  • If ${CURRENT} (or higher) is already on crates.io / PyPI, do not
    re-publish a lower number — bump forward instead.

EOF
  confirm_or_abort "Proceed with downgrade to ${NEW}?"
fi

if [[ "$DO_CHANGELOG" -eq 1 && "$DIRECTION" != "downgrade" ]]; then
  maybe_stub_changelog "$NEW"
elif [[ "$DO_CHANGELOG" -eq 1 && "$DIRECTION" == "downgrade" ]]; then
  echo "Skipping --changelog on downgrade (will not remove or rewrite history)."
  if grep -Eq "^## \[${NEW}\]" CHANGELOG.md 2>/dev/null; then
    echo "CHANGELOG.md already has ## [${NEW}]"
  else
    echo "warning: CHANGELOG.md has no ## [${NEW}] section; add one manually if needed."
  fi
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Dry-run complete. Re-run without --dry-run to apply."
  exit 0
fi

# Always rewrite to NEW (including sync when NEW == CURRENT).
bump_cargo_toml "$NEW"
bump_python_pyprojects "$NEW"
bump_r_bindings "$NEW"

echo "Updated. Consistency check:"
echo
if report_versions "$NEW"; then
  echo "All lockstep sites match ${NEW}."
else
  echo "warning: some sites still disagree; inspect with git diff" >&2
fi

echo
echo "Next steps:"
echo "  1. Review:  git diff"
if [[ "$DIRECTION" == "downgrade" ]]; then
  echo "  2. If you added a CHANGELOG section for the higher version by mistake, edit/remove it by hand."
  echo "  3. If you created a local tag for the higher version:  git tag -d v${CURRENT}"
  echo "  4. Commit the corrected version on your release-prep branch"
elif [[ "$DIRECTION" == "sync" ]]; then
  echo "  2. Commit the re-synced metadata if the drift was unintentional"
else
  if [[ "$DO_CHANGELOG" -eq 0 ]]; then
    echo "  2. Ensure CHANGELOG.md has:  ## [${NEW}]"
    echo "     (or re-run with --changelog for a stub)"
  else
    echo "  2. Fill in CHANGELOG.md bullets under ## [${NEW}]"
  fi
  echo "  3. Commit on release/v${NEW} (or your release-prep branch)"
  echo "  4. After merge to main:  git tag -a v${NEW} -m v${NEW} && git push origin v${NEW}"
fi
echo
echo "Member crates use version.workspace = true — no per-crate Cargo.toml edits needed."
echo "Accidentally too high?  ./scripts/bump-version.sh set <lower> [--yes]"
