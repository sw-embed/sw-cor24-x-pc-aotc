#!/bin/bash
# run-tests.sh — Compile all test Pascal programs, run under both pvm.s
# and pv24t, compare outputs, and save golden files.
#
# Usage: ./scripts/run-tests.sh [--trace]
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
SW_EMBED="$(cd "$REPO/.." && pwd)"

PV24T="$SW_EMBED/sw-cor24-pcode/target/release/pv24t"
PVM="$SW_EMBED/sw-cor24-pcode/vm/pvm.s"
RELOCATE="$SW_EMBED/sw-cor24-pascal/scripts/relocate_p24.py"
COMPILE="$REPO/scripts/compile-test.sh"

TESTS_DIR="$REPO/tests/pascal"
GOLDEN_DIR="$REPO/tests/golden"
P24_DIR="$REPO/tests/p24"
TRACE_DIR="$REPO/tests/traces"

TRACE_FLAG=""
if [[ "${1:-}" == "--trace" ]]; then
  TRACE_FLAG="-t"
fi

mkdir -p "$GOLDEN_DIR" "$P24_DIR" "$TRACE_DIR"

# Build pv24t in release mode
echo "=== Building pv24t ==="
(cd "$SW_EMBED/sw-cor24-pcode" && cargo build --release -p pv24t 2>&1 | tail -1)

PASS=0
FAIL=0
SKIP=0

for PAS in "$TESTS_DIR"/*.pas; do
  NAME=$(basename "$PAS" .pas)
  P24="$P24_DIR/$NAME.p24"
  GOLDEN="$GOLDEN_DIR/$NAME.expected"
  TRACE="$TRACE_DIR/$NAME.trace"

  printf "%-25s " "$NAME..."

  # Compile
  if ! bash "$COMPILE" "$PAS" "$P24" >/dev/null 2>&1; then
    echo "SKIP (compile failed)"
    SKIP=$((SKIP + 1))
    continue
  fi

  # Run under pv24t (reference interpreter)
  TRACE_ARGS=""
  if [[ -n "$TRACE_FLAG" ]]; then
    TRACE_ARGS="-t"
  fi
  PV24T_OUT=$("$PV24T" $TRACE_ARGS -n 10000000 "$P24" 2>"$TRACE" || true)

  # Run under pvm.s (hardware interpreter) for cross-validation
  TMP="/tmp/pvmrun_$$"
  mkdir -p "$TMP"

  # Relocate for pvm.s
  cp "$P24" "$TMP/$NAME.p24"
  python3 "$RELOCATE" "$TMP/$NAME.p24" 0x010000 >/dev/null 2>&1
  printf '\x00\x00\x01' > "$TMP/code_ptr.bin"

  PVM_OUT=$(cor24-run --run "$PVM" \
    --load-binary "$TMP/$NAME.bin@0x010000" \
    --load-binary "$TMP/code_ptr.bin@0x0A12" \
    --terminal --speed 0 -n 50000000 2>&1 | \
    grep -v '^\[' | grep -v '^Assembled' | grep -v '^Running' | \
    grep -v '^Executed' | grep -v '^Loaded' | grep -v '^PVM OK' | \
    grep -v '^$' | grep -v '^HALT$' || true)

  rm -rf "$TMP"

  # Compare
  if [[ "$PV24T_OUT" == "$PVM_OUT" ]]; then
    echo "$PV24T_OUT" > "$GOLDEN"
    echo "PASS"
    PASS=$((PASS + 1))
  else
    echo "FAIL (pv24t vs pvm.s differ)"
    echo "  pv24t: $(echo "$PV24T_OUT" | head -3)"
    echo "  pvm.s: $(echo "$PVM_OUT" | head -3)"
    # Save both for inspection
    echo "$PV24T_OUT" > "$GOLDEN_DIR/$NAME.pv24t"
    echo "$PVM_OUT" > "$GOLDEN_DIR/$NAME.pvm"
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "=== Results: $PASS passed, $FAIL failed, $SKIP skipped ==="

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
