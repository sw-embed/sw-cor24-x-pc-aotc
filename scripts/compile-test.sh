#!/bin/bash
# compile-test.sh — Compile a Pascal test program to .p24
# Usage: ./scripts/compile-test.sh <file.pas> <output.p24>
#
# Pipeline: .pas → p24p → .spc → pl24r → pa24r → .p24
set -euo pipefail

PAS="${1:?Usage: $0 <file.pas> <output.p24>}"
OUT="${2:?Usage: $0 <file.pas> <output.p24>}"

# Tool paths (relative to sw-embed)
SW_EMBED="$(cd "$(dirname "$0")/.." && pwd)/.."
P24P_S="$SW_EMBED/sw-cor24-pascal/compiler/p24p.s"
PL24R="$SW_EMBED/sw-cor24-pcode/target/release/pl24r"
PA24R="$SW_EMBED/sw-cor24-pcode/target/release/pa24r"
RUNTIME="$SW_EMBED/sw-cor24-pascal/runtime/runtime.spc"

NAME=$(basename "$PAS" .pas)
TMP="/tmp/p24p_compile_$$"
mkdir -p "$TMP"
trap "rm -rf $TMP" EXIT

# Step 1: Compile Pascal to .spc
SPC_OUTPUT=$(printf '%s\x04' "$(cat "$PAS")" | \
  cor24-run --run "$P24P_S" --terminal --speed 0 -n 50000000 2>&1)

if ! echo "$SPC_OUTPUT" | grep -q "; OK"; then
  echo "COMPILE FAILED: $PAS" >&2
  echo "$SPC_OUTPUT" | grep -i "error" >&2
  exit 1
fi

echo "$SPC_OUTPUT" | sed -n '/^\.module/,/^\.endmodule/p' > "$TMP/$NAME.spc"

# Step 2: Link with runtime
"$PL24R" "$RUNTIME" "$TMP/$NAME.spc" -o "$TMP/${NAME}_linked.spc" 2>/dev/null

# Step 3: Assemble to .p24
"$PA24R" "$TMP/${NAME}_linked.spc" -o "$OUT" 2>/dev/null

echo "OK: $PAS → $OUT"
