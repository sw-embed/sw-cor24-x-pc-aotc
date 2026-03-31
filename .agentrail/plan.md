# COR24 P-Code AOT Compiler (pc-aotc)

Implement an ahead-of-time compiler that translates p-code bytecode into
COR24 native assembly. Staged approach from research.txt:

## Stages
1. Formalize VM/runtime contracts — document p-code semantics, frame
   layout, calling convention, runtime helper ABI, memory regions,
   error behavior
2. Interpreter tracing — add tracing mode to pv24a for differential
   testing, create deterministic test programs
3. Cargo workspace — set up Rust crates: pcode-model, pcode-analyze,
   pcode-lower, cor24-emit-asm, pcode-aotc
4. P-code decoder — parse p-code binary format, decode instructions,
   define opcode enum and instruction struct
5. Direct lowering subset — p-code → .s for tiny subset (LIT, load/store,
   arithmetic, comparisons, jumps)
6. Procedure calls — activation record setup, parameter passing,
   runtime helper calls
7. Differential test harness — compare interpreter vs native execution
8. Control flow — while/for/case/if-else, basic block analysis,
   stack-depth propagation
9. Arrays and records — bounds checking, field offsets, array indexing
10. IR layer — graduate to IR-based lowering with proper IR instruction set
11. Optimizations — constant folding, dead temp elimination, TOS caching,
    branch simplification
12. Heap and runtime — heap alloc, strings, sets, remaining helpers
13. Full validation — comprehensive differential testing, benchmark
14. Documentation and release
