#!usr/bin/bash
set -euo pipefail

py_version="3.12"

crates=(
    biosym
    cores
    loadsym
    runsym
)

for crate in "${crates[@]}"; do
    echo "Creating venv for $crate..."
    python${py_version} -m venv "../${crate}/.venv"
done

echo "All virtual environments created."
