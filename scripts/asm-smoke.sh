#!/usr/bin/env bash
# x86_64 hotpath asm smoke — Linux GAS + C reference parity.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BUILD="$ROOT/target/asm-smoke"
mkdir -p "$BUILD"

if [[ "$(uname -m)" != "x86_64" ]]; then
  echo "skip asm smoke: host is not x86_64"
  exit 0
fi

echo "==> compile asm hotpath test (linux GAS)"
gcc -O2 -Wall -Werror \
  -DSKW_BUILDING_MODULE -DSKW_HOTPATH_ASM \
  -I"$ROOT/c/include" \
  "$ROOT/c/src/hotpath/dispatch.c" \
  "$ROOT/c/src/runtime/value.c" \
  "$ROOT/asm/x86_64/linux/hash.S" \
  "$ROOT/asm/x86_64/linux/i64.S" \
  "$ROOT/asm/x86_64/linux/tagged.S" \
  "$ROOT/tests/ffi/asm_hotpath_test.c" \
  -o "$BUILD/asm_hotpath_test"

"$BUILD/asm_hotpath_test"

echo "==> C-only fallback (no asm symbols)"
gcc -O2 -Wall -Werror \
  -DSKW_BUILDING_MODULE \
  -I"$ROOT/c/include" \
  "$ROOT/c/src/hotpath/dispatch.c" \
  "$ROOT/tests/ffi/asm_hotpath_test.c" \
  -o "$BUILD/asm_hotpath_c_test"

"$BUILD/asm_hotpath_c_test"

echo "asm smoke ok"
