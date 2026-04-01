#!/bin/bash
# run_diff_tests.sh — Differential test harness for pc-aotc
#
# Compares AOT-compiled native output against the p-code interpreter.
# For each .p24 test file:
#   1. Run under pv24t interpreter → expected output
#   2. AOT compile to .s with pc-aotc
#   3. Assemble and run with cor24-run → actual output
#   4. Compare stdout
#
# Usage:
#   ./tests/run_diff_tests.sh              # Run all tests
#   ./tests/run_diff_tests.sh arithmetic   # Run specific test
#   ./tests/run_diff_tests.sh --compile    # Also compile .pas → .p24 first
#
# Environment variables:
#   PV24T     Path to p-code interpreter (default: auto-detect)
#   PC_AOTC   Path to pc-aotc binary (default: cargo build)
#   COR24_RUN Path to cor24-run (default: auto-detect)
#   P24P_S    Path to p24p.s compiler (needed with --compile)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
P24_DIR="$REPO_DIR/tests/p24"
PAS_DIR="$REPO_DIR/tests/pascal"
TMP_DIR="${TMPDIR:-/tmp}/pc-aotc-diff-$$"
mkdir -p "$TMP_DIR"
trap 'rm -rf "$TMP_DIR"' EXIT

# --- Tool discovery ---

find_tool() {
    local name="$1"
    local var_name="$2"
    local search_paths=("${@:3}")

    # Check environment variable
    if [ -n "${!var_name:-}" ]; then
        if [ -x "${!var_name}" ]; then
            echo "${!var_name}"
            return 0
        fi
        echo "WARNING: $var_name=${!var_name} not executable" >&2
    fi

    # Check PATH
    if command -v "$name" >/dev/null 2>&1; then
        command -v "$name"
        return 0
    fi

    # Check known locations
    for path in "${search_paths[@]}"; do
        if [ -x "$path" ]; then
            echo "$path"
            return 0
        fi
    done

    return 1
}

PV24T_BIN=$(find_tool pv24t PV24T \
    "$HOME/github/sw-embed/sw-cor24-pcode/target/release/pv24t" \
    "$HOME/github/sw-embed/sw-cor24-pcode/target/debug/pv24t" \
) || { echo "SKIP: pv24t not found (set PV24T=<path>)"; exit 0; }

COR24_RUN_BIN=$(find_tool cor24-run COR24_RUN \
    "$HOME/.local/softwarewrighter/bin/cor24-run" \
) || { echo "SKIP: cor24-run not found (set COR24_RUN=<path>)"; exit 0; }

# Build pc-aotc if needed
if [ -n "${PC_AOTC:-}" ] && [ -x "${PC_AOTC}" ]; then
    PC_AOTC_BIN="$PC_AOTC"
else
    echo "Building pc-aotc..."
    cargo build -p pcode-aotc --manifest-path "$REPO_DIR/Cargo.toml" 2>&1 | tail -1
    PC_AOTC_BIN="$REPO_DIR/target/debug/pcode-aotc"
fi

echo "Tools:"
echo "  pv24t:     $PV24T_BIN"
echo "  pc-aotc:   $PC_AOTC_BIN"
echo "  cor24-run: $COR24_RUN_BIN"
echo ""

# --- Parse arguments ---

COMPILE_PAS=false
FILTER=""
VERBOSE=false

for arg in "$@"; do
    case "$arg" in
        --compile) COMPILE_PAS=true ;;
        --verbose|-v) VERBOSE=true ;;
        --help|-h)
            echo "Usage: $0 [--compile] [--verbose] [test_name ...]"
            exit 0
            ;;
        *) FILTER="$FILTER $arg" ;;
    esac
done

# --- Pascal compilation (optional) ---

compile_pascal() {
    local pas_file="$1"
    local name=$(basename "$pas_file" .pas)
    local p24p_s="${P24P_S:-$HOME/github/sw-embed/sw-cor24-pascal/compiler/p24p.s}"
    local pl24r="$HOME/github/sw-embed/sw-cor24-pcode/target/release/pl24r"
    local pa24r="$HOME/github/sw-embed/sw-cor24-pcode/target/release/pa24r"
    local runtime="$HOME/github/sw-embed/sw-cor24-pascal/runtime/runtime.spc"

    for tool in "$p24p_s" "$pl24r" "$pa24r" "$runtime"; do
        if [ ! -f "$tool" ]; then
            echo "  SKIP compile: $(basename "$tool") not found"
            return 1
        fi
    done

    local ctmp="$TMP_DIR/compile_$name"
    mkdir -p "$ctmp"

    # Compile Pascal to .spc
    local spc_output
    spc_output=$(printf '%s\x04' "$(cat "$pas_file")" | \
        "$COR24_RUN_BIN" --run "$p24p_s" --terminal --speed 0 -n 10000000 2>&1)

    if ! echo "$spc_output" | grep -q "; OK"; then
        echo "  SKIP: p24p compilation failed"
        return 1
    fi

    echo "$spc_output" | sed -n '/^\.module/,/^\.endmodule/p' > "$ctmp/$name.spc"

    # Link with runtime
    "$pl24r" "$runtime" "$ctmp/$name.spc" -o "$ctmp/${name}_linked.spc" 2>/dev/null

    # Assemble to .p24
    "$pa24r" "$ctmp/${name}_linked.spc" -o "$ctmp/$name.p24" 2>/dev/null

    cp "$ctmp/$name.p24" "$P24_DIR/$name.p24"
    return 0
}

# --- Test execution ---

passed=0
failed=0
skipped=0
errors=""

run_test() {
    local p24_file="$1"
    local name=$(basename "$p24_file" .p24)

    printf "%-20s " "$name"

    # Step 1: Run under interpreter
    local expected
    expected=$("$PV24T_BIN" "$p24_file" 2>/dev/null) || {
        echo "SKIP (interpreter error)"
        skipped=$((skipped + 1))
        return
    }

    # Step 2: AOT compile to .s
    local s_file="$TMP_DIR/$name.s"
    local aotc_stderr
    aotc_stderr=$("$PC_AOTC_BIN" "$p24_file" -o "$s_file" 2>&1) || {
        echo "FAIL (aotc error)"
        if $VERBOSE; then
            echo "    $aotc_stderr"
        fi
        failed=$((failed + 1))
        errors="$errors\n--- $name: AOT compile error ---\n$aotc_stderr\n"
        return
    }

    # Step 3: Assemble and run with cor24-run
    local actual
    local run_output
    run_output=$("$COR24_RUN_BIN" --run "$s_file" --speed 0 --time 10 2>&1) || true

    # Check for assembly errors
    if echo "$run_output" | grep -q "^Assembly errors:"; then
        echo "FAIL (assembly errors)"
        if $VERBOSE; then
            echo "$run_output" | head -10 | sed 's/^/    /'
        fi
        failed=$((failed + 1))
        local asm_errors
        asm_errors=$(echo "$run_output" | grep -c "not supported\|Invalid" || true)
        errors="$errors\n--- $name: Assembly failed ($asm_errors errors) ---\n"
        if $VERBOSE; then
            errors="$errors$(echo "$run_output" | head -20)\n"
        fi
        return
    fi

    # Extract program output (filter emulator status lines)
    actual=$(echo "$run_output" | grep -v '^\[' | grep -v '^Assembled' | \
        grep -v '^Running' | grep -v '^Executed' | grep -v '^Loaded' | \
        grep -v '^$' | grep -v '^HALT$' || true)

    # Step 4: Compare
    if [ "$expected" = "$actual" ]; then
        echo "PASS"
        passed=$((passed + 1))
    else
        echo "FAIL (output mismatch)"
        failed=$((failed + 1))
        local diff_output
        diff_output=$(diff <(echo "$expected") <(echo "$actual") || true)
        errors="$errors\n--- $name: Output mismatch ---\n"
        errors="$errors  Expected:\n$(echo "$expected" | head -10 | sed 's/^/    /')\n"
        errors="$errors  Actual:\n$(echo "$actual" | head -10 | sed 's/^/    /')\n"
        errors="$errors  Diff:\n$(echo "$diff_output" | head -20 | sed 's/^/    /')\n"
    fi
}

# --- Main ---

echo "=== pc-aotc Differential Tests ==="
echo ""

# Optionally compile Pascal sources first
if $COMPILE_PAS; then
    echo "Compiling Pascal sources..."
    for pas_file in "$PAS_DIR"/*.pas; do
        name=$(basename "$pas_file" .pas)
        if [ -n "$FILTER" ] && ! echo "$FILTER" | grep -qw "$name"; then
            continue
        fi
        printf "  %-20s " "$name.pas"
        if compile_pascal "$pas_file"; then
            echo "OK"
        fi
    done
    echo ""
fi

# Run differential tests
echo "Running tests..."
echo ""

for p24_file in "$P24_DIR"/*.p24; do
    name=$(basename "$p24_file" .p24)
    if [ -n "$FILTER" ] && ! echo "$FILTER" | grep -qw "$name"; then
        continue
    fi
    run_test "$p24_file"
done

# Summary
echo ""
echo "=== Results ==="
total=$((passed + failed + skipped))
echo "Total: $total  Passed: $passed  Failed: $failed  Skipped: $skipped"

if [ -n "$errors" ]; then
    echo ""
    echo "=== Failure Details ==="
    printf "$errors"
fi

if [ $failed -gt 0 ]; then
    exit 1
fi
exit 0
