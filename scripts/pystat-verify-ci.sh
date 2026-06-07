#!/usr/bin/env bash
# PyStat CI preset — verify fixtures against golden manifests.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo run --release --quiet -- pystat verify --preset ci --all
