# Step 002: Interpreter Tracing

## What was done

1. **Built pv24t** — a Rust-based p-code trace interpreter added to the
   sw-cor24-pcode workspace. Rather than modifying the COR24 assembly
   interpreter (pvm.s), a clean Rust implementation was written that:
   - Reads .p24 binaries directly using the existing `pa24r` library
   - Implements all 49 opcodes with identical semantics to pvm.s
   - Supports `-t` flag for instruction-level tracing to stderr
   - Logs: p-code PC, opcode, operands, eval stack depth/values, frame pointer
   - Supports `-n <count>` instruction limit and `-i <text>` stdin input

2. **Created 6 deterministic test programs** in `tests/pascal/`:
   - `arithmetic.pas` — integer arithmetic (+, -, *, div, mod, negation, compound)
   - `locals.pas` — local variable assignment, computation, swapping
   - `if_else.pas` — conditionals, nested if, chained else-if
   - `while_loop.pas` — while loops, for-to, for-downto, nested loops
   - `procedure_call.pas` — runtime procedure calls (write, writeln)
   - `recursion.pas` — iterative factorial and fibonacci

   Note: p24p Phase 1 doesn't support user-defined procedures/functions,
   so procedure_call.pas and recursion.pas use iterative approaches
   with runtime library calls.

3. **Created test infrastructure** in `scripts/`:
   - `compile-test.sh` — compiles .pas → .p24 through full pipeline
   - `run-tests.sh` — compiles all tests, runs under both pv24t and pvm.s,
     compares outputs, saves golden files and trace logs

4. **Generated golden output files** in `tests/golden/`:
   - All 6 tests produce matching output between pv24t and pvm.s
   - Trace files capture full instruction-level execution state

## Design decisions

- **Rust tracer vs COR24 assembly modification**: Writing tracing in COR24
  assembly would be extremely tedious and fragile. A Rust tracer is
  maintainable, testable, and serves as an independent reference
  implementation for differential testing.

- **pv24t lives in sw-cor24-pcode**: It depends on the pa24r library for
  .p24 file loading and opcode definitions, so the pcode workspace is
  the natural home.

- **Output-level golden files**: The primary validation is program output
  matching between interpreters. Instruction-level traces are for
  debugging, not regression testing.
