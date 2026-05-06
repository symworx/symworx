#!/usr/bin/env bash
set -euo pipefail

crates=(
    biosym
    core
    loadsym
    runsym
)

for crate in "${crates[@]}"; do
    echo "Running cargo clippy for $crate..."
    cargo clippy -p "$crate" --all-targets --all-features
done

echo "All crates tested; see results above."
