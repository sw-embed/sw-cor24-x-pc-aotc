# sw-cor24-x-pc-aotc

Ahead-of-time compiler from COR24 p-code to COR24 native assembly.

## What This Is

This tool translates p-code bytecode (produced by the Pascal compiler
`p24p`) into COR24 `.s` assembly files. The output feeds into the
existing assembler/linker toolchain to produce native COR24 binaries —
no p-code interpreter needed at runtime.

**AOT (ahead-of-time) compilation** means the translation from p-code
to native code happens before execution, on the host machine, producing
a static `.s` file. This contrasts with a JIT compiler which would
translate at runtime on the target hardware.

## Status

**In development** — direct lowering for core p-code subset implemented.
P-code decoder complete; assembly emitter handles stack ops, arithmetic,
comparisons, control flow, frame setup, and local/global variable access.
CLI reads `.p24` files and writes `.s` assembly output.

## Naming Convention

| Prefix | Meaning | Example |
|--------|---------|---------|
| `sw-cor24-` | Software targeting COR24 ecosystem | `sw-cor24-pcode` |
| `sw-cor24-x-` | Cross-tool written in Rust, runs on host | `sw-cor24-x-pc-aotc` (this repo) |
| `web-sw-cor24-` | Browser/UI tools for COR24 | `web-sw-cor24-dashboard` |
| `hw-` | Hardware projects | `hw-cor24-fpga` |

The `x-` prefix indicates a cross-tool: it runs on the host machine
(Mac/Linux), not on COR24 hardware. This repo is written in Rust.

## Compilation Pipeline

```
.pas → p24p → .p24 → pc-aotc → .s → as24 → binary
 │      │      │       │         │     │       │
 │      │      │       │         │     │       └─ Native COR24 executable
 │      │      │       │         │     └─ Cross-assembler (sw-cor24-x-assembler)
 │      │      │       │         └─ COR24 assembly text
 │      │      │       └─ This tool (AOT compiler)
 │      │      └─ P-code bytecode file
 │      └─ Pascal compiler (sw-cor24-pascal)
 └─ Pascal source
```

## Related Repositories

All repos live under `~/github/sw-embed/` as siblings:

| Repository | Description |
|------------|-------------|
| `sw-cor24-pcode` | P-code VM (`pv24a`), assembler (`pa24r`), linker (`pl24r`) — the interpreter this AOT compiler targets |
| `sw-cor24-pascal` | Pascal compiler (`p24p`) that produces the p-code input |
| `sw-cor24-x-assembler` | Cross-assembler in Rust (`as24`) — assembles the `.s` output |
| `sw-cor24-emulator` | Emulator for running and testing native COR24 binaries |
| `sw-cor24-assembler` | Native COR24 assembler in C (runs on COR24 hardware) |
| `sw-cor24-x-tinyc` | Cross C compiler in Rust (`tc24r`) |

## License

MIT License — see [LICENSE](LICENSE) for details.

Copyright (c) 2026 Michael A Wright
