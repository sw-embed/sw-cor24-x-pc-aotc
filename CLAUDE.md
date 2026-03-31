# sw-cor24-x-pc-aotc — Claude Instructions

## Project Overview

AOT (ahead-of-time) compiler that translates p-code bytecode into COR24
native assembly (`.s` files). Written in Rust, runs on the host machine.
The output feeds into the existing cross-assembler/linker toolchain to
produce native COR24 binaries — no p-code interpreter needed at runtime.

The compiler uses a staged approach: direct lowering first (p-code → .s),
graduating to IR-based lowering later. Runtime helpers (write_int,
heap_alloc, bounds_check, etc.) are called from generated code, not
inlined. The p-code interpreter (`pv24a`) remains the canonical
reference forever; differential testing against it is the core
validation strategy.

## CRITICAL: AgentRail Session Protocol (MUST follow exactly)

### 1. START (do this FIRST, before anything else)
```bash
agentrail next
```
Read the output carefully. It contains your current step, prompt,
plan context, and any relevant skills/trajectories.

### 2. BEGIN (immediately after reading the next output)
```bash
agentrail begin
```

### 3. WORK (do what the step prompt says)
Do NOT ask "want me to proceed?". The step prompt IS your instruction.
Execute it directly.

### 4. COMMIT (after the work is done)
Commit your code changes with git. Use `/mw-cp` for the checkpoint
process (pre-commit checks, docs, detailed commit, push).

### 5. COMPLETE (LAST thing, after committing)
```bash
agentrail complete --summary "what you accomplished" \
  --reward 1 \
  --actions "tools and approach used"
```
- If the step failed: `--reward -1 --failure-mode "what went wrong"`
- If the saga is finished: add `--done`

### 6. STOP (after complete, DO NOT continue working)
Do NOT make further code changes after running `agentrail complete`.
Any changes after complete are untracked and invisible to the next
session. Future work belongs in the NEXT step, not this one.

## Key Rules

- **Do NOT skip steps** — the next session depends on accurate tracking
- **Do NOT ask for permission** — the step prompt is the instruction
- **Do NOT continue working** after `agentrail complete`
- **Commit before complete** — always commit first, then record completion

## Useful Commands

```bash
agentrail status          # Current saga state
agentrail history         # All completed steps
agentrail plan            # View the plan
agentrail next            # Current step + context
```

## Build / Test

```bash
# Build all crates
cargo build

# Run all tests
cargo test

# Run a specific crate's tests
cargo test -p pcode-model

# AOT compile a p-code file to assembly
cargo run -- input.p24 -o output.s

# Differential testing (once harness exists):
# 1. Compile Pascal to p-code:  p24p test.pas → test.p24
# 2. Run under interpreter:     pv24a test.p24 > expected.txt
# 3. AOT compile:               pc-aotc test.p24 -o test.s
# 4. Assemble + link:           as24 test.s → test.bin
# 5. Run native:                cor24-run test.bin > actual.txt
# 6. Compare:                   diff expected.txt actual.txt
```

## Architecture

```
Front End          Middle End         Back End
─────────          ──────────         ────────
p-code file  →  decoded procs  →  lowered IR  →  COR24 .s
             →  CFG             →  basic blocks
             →  stack depth     →  optimizations
```

### Crate Structure

| Crate | Responsibility |
|-------|---------------|
| `pcode-model` | Opcode definitions, decoded instructions, procedure metadata |
| `pcode-analyze` | CFG formation, stack-depth analysis, validation |
| `pcode-lower` | Lowering from p-code to simple IR |
| `cor24-emit-asm` | COR24 assembly emission from lowered IR |
| `pcode-aotc` | CLI binary |

### COR24 Register Constraints

| Register | Name | Purpose |
|----------|------|---------|
| r0 | — | Scratch / TOS cache / return value |
| r1 | — | Scratch / NOS cache |
| r2 | — | Scratch |
| r3 | fp | Frame pointer |
| r4 | sp | Stack pointer |
| r5 | z | Zero register |
| r6 | iv | Interrupt vector |
| r7 | ir | Interrupt return |

Register allocation: TOS caching in r0, NOS optionally in r1/r2,
everything else in memory (frame slots / eval stack).

## Cross-Repo Context

All COR24 repos live under `~/github/sw-embed/` as siblings:
- `sw-cor24-pcode` — p-code VM (pv24a), assembler (pa24r), linker (pl24r)
- `sw-cor24-pascal` — Pascal compiler (p24p) producing p-code input
- `sw-cor24-x-assembler` — cross-assembler in Rust (as24)
- `sw-cor24-emulator` — emulator for running/testing native binaries
- `sw-cor24-assembler` — native assembler in C (cas24)
- `sw-cor24-x-tinyc` — cross C compiler in Rust (tc24r)
