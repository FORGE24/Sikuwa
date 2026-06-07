#!/usr/bin/env bash
# Plan 8d — multi-module build smoke (plan5_caller imports add).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BUILD="$ROOT/target/multimodule-smoke"
OUT="$BUILD/dist"
rm -rf "$BUILD"
mkdir -p "$OUT"

echo "==> sikuwa build plan5_caller.py"
cargo run --quiet -- build tests/fixtures/plan5_caller.py -o "$OUT" --opt

SO="$OUT/libplan5_caller.so"
if [[ "$(uname -s)" == "Darwin" ]]; then
  SO="$OUT/libplan5_caller.dylib"
fi
if [[ ! -f "$SO" ]]; then
  echo "missing shared library: $SO" >&2
  exit 1
fi

echo "==> compile & run multimodule harness"
gcc -O2 -Wall -Werror -I"$ROOT/c/include" \
  -I"$OUT/add" -I"$OUT/plan5_caller" \
  "$ROOT/tests/ffi/multimodule_harness.c" "$SO" -o "$OUT/harness"
"$OUT/harness"
echo "multimodule smoke ok"
