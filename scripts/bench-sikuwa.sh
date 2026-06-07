#!/usr/bin/env bash
# Sikuwa 2.0 — functional smoke + compile/runtime benchmark (Linux/macOS).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

step() {
  local label="$1"
  shift
  local start end ms
  start=$(date +%s%3N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1000))')
  "$@"
  end=$(date +%s%3N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1000))')
  ms=$((end - start))
  printf "  %-42s %8d ms\n" "$label" "$ms"
}

BUILD="$ROOT/target/sikuwa-bench"
OUT="$BUILD/out"
rm -rf "$BUILD"
mkdir -p "$OUT"

echo ""
echo "=== Sikuwa benchmark $(date '+%Y-%m-%d %H:%M:%S') ==="
echo ""

echo "[1/4] Release toolchain"
step "cargo build --release" cargo build --release --quiet
cargo run --release --quiet -- version

echo ""
echo "[2/4] CLI compile-time"
step "codegen add.py"           cargo run --release --quiet -- codegen c tests/fixtures/add.py --out-dir "$OUT/add"
step "codegen add.py --opt"     cargo run --release --quiet -- codegen c tests/fixtures/add.py --out-dir "$OUT/add-opt" --opt
step "codegen sum_range --opt"  cargo run --release --quiet -- codegen c tests/fixtures/sum_range.py --out-dir "$OUT/sum_range" --opt
step "codegen clamp --opt"      cargo run --release --quiet -- codegen c tests/fixtures/clamp.py --out-dir "$OUT/clamp" --opt
step "codegen plan3 --opt"      cargo run --release --quiet -- codegen c tests/fixtures/plan3.py --out-dir "$OUT/plan3" --opt
step "pystat report add"        cargo run --release --quiet -- pystat report tests/fixtures/add.py
step "pystat verify ci --all"   cargo run --release --quiet -- pystat verify --preset ci --all
step "build plan5_caller"       cargo run --release --quiet -- build tests/fixtures/plan5_caller.py -o "$OUT/build-dist" --opt

echo ""
echo "[3/4] End-to-end smoke"
for s in ffi-smoke closure-smoke multimodule-smoke asm-smoke; do
  echo "  -> $s"
  bash "$ROOT/scripts/$s.sh"
done

echo ""
echo "[4/4] Native runtime microbench"
SO_ADD="$BUILD/libadd.so"
SO_SUM="$BUILD/libsum_range.so"
[[ "$(uname -s)" == "Darwin" ]] && SO_ADD="$BUILD/libadd.dylib" && SO_SUM="$BUILD/libsum_range.dylib"

step "link libadd"      cargo run --release --quiet -- link shared "$OUT/add-opt" -o "$SO_ADD"
step "link libsum_range" cargo run --release --quiet -- link shared "$OUT/sum_range" -o "$SO_SUM"

gcc -O2 -Wall -Werror -I"$ROOT/c/include" -I"$OUT/add-opt" -I"$OUT/sum_range" \
  -DSKW_BENCH_SUM_RANGE "$ROOT/tests/ffi/bench_harness.c" "$SO_ADD" "$SO_SUM" \
  -o "$BUILD/bench"
"$BUILD/bench"

echo ""
echo "Artifacts: $BUILD"
echo "Done."
