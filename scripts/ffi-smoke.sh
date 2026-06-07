#!/usr/bin/env bash
# Plan 4 FFI smoke — codegen, link, run C harness (Linux/macOS CI).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BUILD="$ROOT/target/ffi-smoke"
mkdir -p "$BUILD/add"

echo "==> codegen add.py"
cargo run --quiet -- codegen c tests/fixtures/add.py --out-dir "$BUILD/add"

SO="$BUILD/libadd.so"
if [[ "$(uname -s)" == "Darwin" ]]; then
  SO="$BUILD/libadd.dylib"
fi

echo "==> link shared library"
cargo run --quiet -- link shared "$BUILD/add" -o "$SO"

echo "==> compile & run harness"
gcc -O2 -Wall -Werror -I"$ROOT/c/include" -I"$BUILD/add" \
  "$ROOT/tests/ffi/harness.c" "$SO" -o "$BUILD/harness"
"$BUILD/harness"

echo "==> ABI header compile check"
gcc -O2 -Wall -Werror -I"$ROOT/c/include" -c "$ROOT/tests/ffi/abi_version.c" -o "$BUILD/abi_version.o"

echo "==> runtime value.c"
gcc -O2 -Wall -Werror -fPIC -DSKW_BUILDING_MODULE -I"$ROOT/c/include" \
  "$ROOT/c/src/runtime/value.c" "$ROOT/tests/ffi/runtime_test.c" -o "$BUILD/runtime_test"
"$BUILD/runtime_test"

echo "==> asm hotpath (x86_64)"
bash "$ROOT/scripts/asm-smoke.sh"

echo "==> clamp codegen (CFG)"
cargo run --quiet -- codegen c tests/fixtures/clamp.py --out-dir "$BUILD/clamp"
grep -q "goto skw_bb_" "$BUILD/clamp/clamp.c"

echo "ffi smoke ok"
