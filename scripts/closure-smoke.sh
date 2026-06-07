#!/usr/bin/env bash
# Plan 8c — closure + class smoke (plan3.py).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BUILD="$ROOT/target/closure-smoke"
mkdir -p "$BUILD/plan3"

echo "==> codegen plan3.py"
cargo run --quiet -- codegen c tests/fixtures/plan3.py --out-dir "$BUILD/plan3"

SO="$BUILD/libplan3.so"
if [[ "$(uname -s)" == "Darwin" ]]; then
  SO="$BUILD/libplan3.dylib"
fi

echo "==> link shared library"
cargo run --quiet -- link shared "$BUILD/plan3" -o "$SO"

echo "==> compile & run closure harness"
gcc -O2 -Wall -Werror -I"$ROOT/c/include" -I"$BUILD/plan3" \
  "$ROOT/tests/ffi/closure_harness.c" "$SO" -o "$BUILD/closure_harness"
"$BUILD/closure_harness"

echo "closure smoke ok"
