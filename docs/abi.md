# COR24 ABI for AOT-Compiled P-Code

Defines the binary interface between AOT-compiled Pascal programs and
the COR24 runtime. This is the contract that both the AOT compiler and
runtime helpers must respect.

Source: `sw-cor24-pcode/vm/pvm.s`, `sw-cor24-pcode/vm/design.md`,
`sw-cor24-pcode/vm/architecture.md`

## COR24 Register Assignments

| Register | Name | AOT Purpose |
|----------|------|-------------|
| r0 | -- | TOS cache / scratch / return value |
| r1 | -- | NOS cache / scratch |
| r2 | -- | Scratch |
| r3 | fp | Frame pointer (points to current activation record) |
| r4 | sp | Stack pointer (COR24 hardware stack in EBR) |
| r5 | z | Zero register (always 0, read-only) |
| r6 | iv | Interrupt vector (reserved) |
| r7 | ir | Interrupt return (reserved) |

### Register Convention

- **r0-r2**: Caller-saved. Any call may destroy them.
- **r3 (fp)**: Callee-saved. Callee must restore before return.
- **r4 (sp)**: Callee-saved (balanced push/pop).
- **r5 (z)**: Read-only zero. Never written.
- **r6-r7**: Reserved for interrupt handling. Never used by compiled code.

### TOS Caching Strategy

The interpreter uses a memory-based eval stack. The AOT compiler should
use TOS caching to avoid excessive memory traffic:

- **r0**: Always holds TOS when the eval stack is non-empty
- **r1**: Optionally holds NOS for binary operations
- **r2**: Available as scratch for address computation

When the eval stack depth exceeds 2, spill to the memory-based eval
stack pointed to by the eval stack pointer (a dedicated memory
location, not a COR24 register).

## Activation Record Layout

The call stack grows upward (toward higher addresses). Each procedure
call creates an activation record (frame) on the call stack.

```
HIGHER ADDRESSES (call stack grows up)

                +-------------------+
                | local[N-1]        |  fp_vm + (N-1)*3
                | ...               |
                | local[0]          |  fp_vm + 0
   fp_vm -----> +-------------------+
                | saved_esp         |  fp_vm - 3    (eval stack at call time)
                | dynamic_link      |  fp_vm - 6    (caller's fp_vm)
                | static_link       |  fp_vm - 9    (enclosing scope's fp_vm)
                | return_pc         |  fp_vm - 12   (return address)
                +-------------------+
                | arg[N-1]          |  saved_esp - N*3
                | ...               |
                | arg[0]            |  saved_esp - 3
                | [eval stack TOS]  |  saved_esp
                +-------------------+

LOWER ADDRESSES
```

### Frame Header (12 bytes)

| Offset from fp_vm | Size | Field | Description |
|--------------------|------|-------|-------------|
| -12 | 3 bytes | return_pc | P-code address to resume after return |
| -9 | 3 bytes | static_link | Frame pointer of lexically enclosing scope |
| -6 | 3 bytes | dynamic_link | Caller's frame pointer (for unwinding) |
| -3 | 3 bytes | saved_esp | Eval stack pointer at time of call |

Note: The interpreter stores these as 4-byte values (word-aligned
reads with `lw`/`sw`), but only the low 3 bytes (24 bits) carry data.
For AOT compilation targeting native COR24, these should be 3 bytes
(one COR24 word) each.

### Local Variables

Local variables occupy slots starting at fp_vm, growing upward.
Each local is one word (3 bytes). Accessed as:

```
local[i] = mem[fp_vm + i * 3]
```

### Argument Access

Arguments are pushed by the caller onto the eval stack before the call.
The callee accesses them relative to saved_esp:

```
arg[i] = mem[saved_esp - (i + 1) * 3]
```

Where i=0 is the first argument (pushed first, deepest on stack).

## Calling Convention

### Call Sequence

1. **Caller** pushes arguments left-to-right onto the eval stack
2. **Caller** executes `call target`:
   - Writes frame header to call stack (return_pc, static_link,
     dynamic_link, saved_esp)
   - Advances call stack pointer by 12 bytes
   - Jumps to target
3. **Callee** executes `enter nlocals`:
   - Sets fp_vm = call stack pointer
   - Advances call stack pointer by nlocals * 3
4. **Callee** does work, accessing args via `loada`, locals via `loadl`
5. **Callee** pushes return value (if function) onto eval stack
6. **Callee** executes `ret nargs`:
   - Detects return value: if eval stack pointer > saved_esp
   - Saves return value if present
   - Restores return_pc, dynamic_link from frame header
   - Pops entire frame from call stack
   - Cleans arguments: eval stack pointer = saved_esp - nargs * 3
   - Pushes return value back if present
   - Jumps to return_pc

### Static Link Setup

For nested procedure calls (`calln depth addr`):
- `depth=0`: static_link = current fp_vm (calling a directly nested proc)
- `depth=1`: static_link = current frame's static_link (calling a sibling)
- `depth>1`: follow static chain (depth-1) times

### Nonlocal Variable Access

To access a variable in an enclosing scope (`loadn depth off`):

```
frame = fp_vm
repeat depth times:
    frame = mem[frame - 9]    // follow static_link
result = mem[frame + off * 3]
```

## Memory Regions

### COR24 Address Space (1 MB SRAM + I/O)

```
0x000000  +------------------------+
          | Code segment           |  Native COR24 instructions
          | (AOT-compiled program  |  (replaces p-code bytecode)
          |  + runtime helpers)    |
          +------------------------+
          | Read-only data         |  String literals, constants
          | (.data sections)       |
          +------------------------+
          | Global variables       |  One word per .global declaration
          +------------------------+
          | Call stack             |  Activation records (grows up)
          +------------------------+
          | Eval stack             |  Expression temporaries (grows up)
          +------------------------+
          | Heap                   |  Dynamic allocation (grows up)
          +------------------------+
          | Free SRAM              |
          +------------------------+
0x0FFFFF  | End of SRAM (1 MB)     |
          +------------------------+
0xFEEC00  | EBR (3 KB)             |  COR24 hardware stack (sp/r4)
0xFEF7FF  +------------------------+
0xFF0000  | LED/Switch I/O         |  Write: LED D2; Read: button S2
0xFF0100  | UART data              |  TX/RX byte
0xFF0101  | UART status            |  Bit 7: TX busy; Bit 0: RX ready
```

### Segment Sizing

The interpreter allocates small fixed segments:
- Globals: 8 words (24 bytes)
- Call stack: 32 words (96 bytes)
- Eval stack: 32 words (96 bytes)
- Heap: 32 words (96 bytes)

For AOT-compiled programs, segment sizes should be configurable.
Reasonable defaults for a 1 MB address space:
- Call stack: 256-1024 words
- Eval stack: 128-512 words
- Heap: remaining free SRAM

### Address Unit

All addresses are byte addresses. One word = 3 bytes. Offsets in
p-code instructions are word indices; multiply by 3 to get byte
offsets.

## Runtime Helper ABI

Runtime helpers are COR24 assembly routines linked with the
AOT-compiled program. They follow the standard COR24 calling
convention using the hardware stack (sp/r4).

### Calling Convention for Helpers

Arguments are passed on the eval stack (matching p-code convention).
The helper pops its arguments and pushes any return value.

| Helper | Args (on eval stack) | Returns | Description |
|--------|---------------------|---------|-------------|
| `_p24p_write_int` | ( n -- ) | nothing | Print signed integer to UART |
| `_p24p_write_bool` | ( flag -- ) | nothing | Print "TRUE" or "FALSE" |
| `_p24p_write_str` | ( addr -- ) | nothing | Print null-terminated string |
| `_p24p_write_ln` | ( -- ) | nothing | Print newline (0x0A) |
| `_p24p_read_int` | ( -- n ) | integer on eval stack | Read integer from UART |
| `_p24p_read_ln` | ( -- ) | nothing | Consume remaining input line |
| `_p24p_led_on` | ( -- ) | nothing | Turn on LED D2 |
| `_p24p_led_off` | ( -- ) | nothing | Turn off LED D2 |

### Helper Implementation Notes

- `_p24p_write_int` converts integer to decimal ASCII, writes digits
  to UART. Handles negative numbers (prefix '-'). Implementation in
  `sw-cor24-pascal/compiler/runtime/phase0.spc`.
- UART TX: write byte to 0xFF0100; poll 0xFF0101 bit 7 until not busy.
- UART RX: poll 0xFF0101 bit 0 until ready; read byte from 0xFF0100.
- LED port at 0xFF0000 is active-low (bit 0 inverted in hardware).

### AOT-Specific Runtime Helpers (Future)

These will be needed as the AOT compiler matures:

| Helper | Args | Returns | Description |
|--------|------|---------|-------------|
| `_rt_bounds_check` | ( idx lo hi -- idx ) | index (unchanged) | Trap if idx < lo or idx > hi |
| `_rt_heap_alloc` | ( size -- ptr ) | pointer | Allocate from heap |
| `_rt_heap_free` | ( ptr -- ) | nothing | Free heap memory (may be no-op) |
| `_rt_div_zero` | ( -- ) | never returns | Trap: division by zero |
| `_rt_nil_deref` | ( -- ) | never returns | Trap: nil pointer dereference |
| `_rt_stack_overflow` | ( -- ) | never returns | Trap: stack overflow |

## Error Behavior

### Trap Codes

| Code | Name | Trigger |
|------|------|---------|
| 0 | USER_TRAP | `trap 0` instruction |
| 1 | DIV_ZERO | `div` or `mod` with divisor = 0 |
| 2 | STACK_OVERFLOW | Eval stack pointer reaches heap boundary |
| 3 | STACK_UNDERFLOW | Pop when eval stack is empty |
| 4 | INVALID_OPCODE | Unrecognized opcode byte (>= 0x61) |
| 5 | INVALID_ADDRESS | Reserved (not implemented in interpreter) |
| 6 | NIL_POINTER | `load`/`store`/`loadb`/`storeb` with address = 0 |
| 7 | BOUNDS_CHECK | Reserved (not implemented in interpreter) |

### Trap Behavior

In the interpreter, a trap sets `status=2` and `trap_code=N`, then
the VM loop exits and prints `TRAP N\n` followed by a halt loop.

For AOT-compiled code, traps should:
1. Print an error message identifying the trap type
2. Print the approximate source location if available
3. Halt execution (enter infinite loop or system halt)

### Division Semantics

- Division truncates toward zero (standard signed integer division)
- Modulo remainder has the same sign as the dividend
- Division and modulo by zero always trap (code 1)

### Nil Pointer Semantics

- Address 0 is the nil pointer
- Any dereference of address 0 traps (code 6)
- This applies to `load`, `store`, `loadb`, `storeb`

### Stack Overflow Detection

- Eval stack overflow: checked on every push; compares eval stack
  pointer against heap segment base address
- Call stack overflow: not explicitly checked in interpreter (implicit
  if call stack grows into eval stack)
- AOT compiler should check call stack depth at procedure entry

## AOT Compilation Considerations

### What Changes from Interpreter to AOT

| Aspect | Interpreter | AOT |
|--------|-------------|-----|
| Dispatch | Fetch-decode-execute loop | Native instructions |
| Eval stack | Memory array + esp pointer | TOS in r0, NOS in r1, spill to memory |
| Call stack | Memory array + csp/fp_vm | Native call frames using fp (r3) |
| Code | p-code bytecode in memory | COR24 native instructions |
| PC | Software counter (vm_state.pc) | Hardware program counter |
| Globals | gp + offset*3 | Fixed addresses resolved at link time |
| Data | Embedded after code section | .data section in assembly output |

### What Stays the Same

- Activation record layout (frame header fields and offsets)
- Argument passing convention (eval stack, left-to-right)
- Return value convention (pushed to eval stack before ret)
- Static link chain traversal for nested scopes
- Runtime helper interface (same symbols, same stack protocol)
- Memory-mapped I/O addresses
- Trap semantics and error behavior
- Word size (3 bytes / 24 bits)
