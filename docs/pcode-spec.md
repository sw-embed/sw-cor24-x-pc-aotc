# P-Code Instruction Set Specification

Canonical reference for the p-code bytecode format used by the COR24
p-code VM (pv24a) and Pascal compiler (p24p). This document is the
source of truth for the AOT compiler's instruction decoder.

Source: `sw-cor24-pcode/vm/pvm.s`, `sw-cor24-pcode/vm/design.md`,
`sw-cor24-pascal/compiler/src/parser.c`

## Word Size and Encoding

- Word size: 3 bytes (24-bit), all values are signed 24-bit integers
- All offsets are word indices (multiply by 3 for byte addresses)
- Opcodes are single bytes; operands follow immediately
- Instructions are byte-aligned, variable length (1-5 bytes)
- Pointers are 24-bit (3 bytes)

## Opcode Table

### Stack and Constants (0x00-0x06)

| Byte | Mnemonic | Size | Stack Effect | Description |
|------|----------|------|--------------|-------------|
| 0x00 | `halt` | 1 | ( -- ) | Stop execution, set status=1 |
| 0x01 | `push` imm24 | 4 | ( -- n ) | Push 24-bit signed immediate |
| 0x02 | `push_s` imm8 | 2 | ( -- n ) | Push 8-bit sign-extended immediate |
| 0x03 | `dup` | 1 | ( a -- a a ) | Duplicate TOS |
| 0x04 | `drop` | 1 | ( a -- ) | Discard TOS |
| 0x05 | `swap` | 1 | ( a b -- b a ) | Exchange TOS and NOS |
| 0x06 | `over` | 1 | ( a b -- a b a ) | Copy NOS to top |

### Arithmetic and Logic (0x10-0x1B)

| Byte | Mnemonic | Size | Stack Effect | Description |
|------|----------|------|--------------|-------------|
| 0x10 | `add` | 1 | ( a b -- a+b ) | Signed addition |
| 0x11 | `sub` | 1 | ( a b -- a-b ) | Signed subtraction |
| 0x12 | `mul` | 1 | ( a b -- a*b ) | Signed multiplication |
| 0x13 | `div` | 1 | ( a b -- a/b ) | Signed division; trap 1 if b=0 |
| 0x14 | `mod` | 1 | ( a b -- a%b ) | Signed modulo; remainder sign = dividend sign; trap 1 if b=0 |
| 0x15 | `neg` | 1 | ( a -- -a ) | Negate (0 - a) |
| 0x16 | `and` | 1 | ( a b -- a&b ) | Bitwise AND |
| 0x17 | `or` | 1 | ( a b -- a\|b ) | Bitwise OR |
| 0x18 | `xor` | 1 | ( a b -- a^b ) | Bitwise XOR |
| 0x19 | `not` | 1 | ( a -- ~a ) | Bitwise complement (a XOR -1) |
| 0x1A | `shl` | 1 | ( a n -- a<<n ) | Left shift by n |
| 0x1B | `shr` | 1 | ( a n -- a>>n ) | Arithmetic right shift by n |

Note: Division and modulo are implemented via repeated subtraction on
absolute values; sign is computed separately and applied afterward.

### Comparison (0x20-0x25)

All comparisons are signed; push 1 (true) or 0 (false).

| Byte | Mnemonic | Size | Stack Effect | Description |
|------|----------|------|--------------|-------------|
| 0x20 | `eq` | 1 | ( a b -- flag ) | a == b |
| 0x21 | `ne` | 1 | ( a b -- flag ) | a != b |
| 0x22 | `lt` | 1 | ( a b -- flag ) | a < b |
| 0x23 | `le` | 1 | ( a b -- flag ) | a <= b |
| 0x24 | `gt` | 1 | ( a b -- flag ) | a > b |
| 0x25 | `ge` | 1 | ( a b -- flag ) | a >= b |

### Control Flow (0x30-0x36)

| Byte | Mnemonic | Size | Stack Effect | Description |
|------|----------|------|--------------|-------------|
| 0x30 | `jmp` addr24 | 4 | ( -- ) | Unconditional jump to addr24 |
| 0x31 | `jz` addr24 | 4 | ( flag -- ) | Jump if TOS == 0 (pop flag) |
| 0x32 | `jnz` addr24 | 4 | ( flag -- ) | Jump if TOS != 0 (pop flag) |
| 0x33 | `call` addr24 | 4 | ( args... -- ) | Call procedure at addr24 |
| 0x34 | `ret` nargs8 | 2 | ( [retval] -- [retval] ) | Return, clean nargs from caller's stack |
| 0x35 | `calln` depth8 addr24 | 5 | ( args... -- ) | Call with static link at depth |
| 0x36 | `trap` code8 | 2 | ( -- ) | Trigger trap with code (0-7) |

### Frame and Local Access (0x40-0x4B)

| Byte | Mnemonic | Size | Stack Effect | Description |
|------|----------|------|--------------|-------------|
| 0x40 | `enter` nlocals8 | 2 | ( -- ) | Set fp_vm=csp, reserve nlocals slots |
| 0x41 | `leave` | 1 | ( -- ) | Restore csp to fp_vm (discard locals) |
| 0x42 | `loadl` off8 | 2 | ( -- val ) | Load local at fp_vm + off*3 |
| 0x43 | `storel` off8 | 2 | ( val -- ) | Store local at fp_vm + off*3 |
| 0x44 | `loadg` off24 | 4 | ( -- val ) | Load global at gp + off*3 |
| 0x45 | `storeg` off24 | 4 | ( val -- ) | Store global at gp + off*3 |
| 0x46 | `addrl` off8 | 2 | ( -- addr ) | Push address of local (fp_vm + off*3) |
| 0x47 | `addrg` off24 | 4 | ( -- addr ) | Push address of global (gp + off*3) |
| 0x48 | `loada` idx8 | 2 | ( -- val ) | Load argument at saved_esp - (idx+1)*3 |
| 0x49 | `storea` idx8 | 2 | ( val -- ) | Store argument at saved_esp - (idx+1)*3 |
| 0x4A | `loadn` depth8 off8 | 3 | ( -- val ) | Load nonlocal: follow static chain depth times, then load local[off] |
| 0x4B | `storen` depth8 off8 | 3 | ( val -- ) | Store nonlocal: follow static chain depth times, then store local[off] |

### Indirect Memory Access (0x50-0x53)

| Byte | Mnemonic | Size | Stack Effect | Description |
|------|----------|------|--------------|-------------|
| 0x50 | `load` | 1 | ( addr -- val ) | Load word from addr; trap 6 if addr=0 |
| 0x51 | `store` | 1 | ( val addr -- ) | Store word to addr; trap 6 if addr=0 |
| 0x52 | `loadb` | 1 | ( addr -- byte ) | Load byte (zero-extended); trap 6 if addr=0 |
| 0x53 | `storeb` | 1 | ( byte addr -- ) | Store byte to addr; trap 6 if addr=0 |

### System Calls (0x60)

| Byte | Mnemonic | Size | Stack Effect | Description |
|------|----------|------|--------------|-------------|
| 0x60 | `sys` id8 | 2 | (varies) | Dispatch system call by id |

System call IDs:

| ID | Name | Stack Effect | Description |
|----|------|--------------|-------------|
| 0 | HALT | ( -- ) | Stop execution (status=1) |
| 1 | PUTC | ( char -- ) | Write byte to UART TX (0xFF0100) |
| 2 | GETC | ( -- char ) | Read byte from UART RX (blocking poll) |
| 3 | LED | ( state -- ) | Write to LED port (0xFF0000); active-low (bit 0 inverted) |
| 4 | ALLOC | ( size -- ptr ) | Bump-allocate: ptr=hp, hp+=size |
| 5 | FREE | ( ptr -- ) | No-op (bump allocator does not free) |
| 6 | READ_SWITCH | ( -- state ) | Read button S2 from 0xFF0000; active-low (bit 0 inverted) |

### Reserved/Invalid Opcodes

Opcodes 0x07-0x0F, 0x1C-0x1F, 0x26-0x2F, 0x37-0x3F, 0x4C-0x4F,
0x54-0x5F, and 0x61+ are invalid and trigger trap code 4
(INVALID_OPCODE).

## P-Code Assembly Source Format (.spc)

The p24p compiler emits `.spc` text files (p-code assembly source).
These are assembled by pasm/pvmasm into bytecode.

### Directives

| Directive | Description |
|-----------|-------------|
| `.module NAME` | Module/program identifier |
| `.export SYMBOL` | Export symbol for linking |
| `.extern SYMBOL` | Declare external symbol |
| `.global NAME NWORDS` | Reserve NWORDS words of global storage |
| `.proc NAME NLOCALS` | Begin procedure with NLOCALS local slots |
| `.end` | End procedure |
| `.data NAME byte,byte,...,0` | Define byte array (null-terminated) |
| `.endmodule` | End module |

### Labels

Labels are local to the enclosing `.proc`/`.end` block. Format:
`L<n>:` where n is a monotonically increasing integer. Resolved to
code offsets during assembly.

### Instruction Format

One instruction per line: `mnemonic [operand]`. Operands may be:
- Decimal integer literals
- Named constants (from `.const`)
- Labels (L0, L1, ...)
- Symbol names (for call, loadg, storeg, addrg targets)

## P-Code Binary Format

P-code bytecode files have no header. The bytecode is a linear byte
stream loaded at a base address. The assembler resolves all symbol
references to absolute addresses.

### Memory Layout After Assembly

```
[Code section]    Bytecode instructions (variable-length)
[Data section]    Byte literals from .data directives (contiguous with code)
```

Globals, call stack, eval stack, and heap are allocated at runtime by
the VM loader, not embedded in the bytecode file.

### Symbol Resolution

- Labels resolve to code offsets (absolute bytecode addresses)
- `.data` names resolve to data section addresses
- `.global` names resolve to global segment addresses (gp + offset*3)
- `.extern` symbols resolved at link time

### No Procedure Table

Unlike some p-code systems, pv24a has no explicit procedure table.
Procedures are labels pointing to code entry points. The `call`
instruction targets a resolved bytecode address.

## Compiler Code Generation Patterns

The Pascal compiler (p24p) produces these characteristic patterns:

### Main Program

```
.proc main 0
    enter 0
    ; ... statements ...
    halt
.end
```

Main always ends with `halt`, never `ret`.

### While Loop

```
L0:                   ; loop top
    <condition>       ; pushes 0 or 1
    jz L1             ; exit if false
    <body>
    jmp L0            ; loop back
L1:                   ; exit
```

### For Loop (to)

```
    <start expr>
    storeg i          ; i := start
L0:
    loadg i
    <limit expr>
    le                ; i <= limit?
    jz L1             ; exit if false
    <body>
    loadg i
    push 1
    add
    storeg i          ; i := i + 1
    jmp L0
L1:
```

For `downto`: uses `ge` and `sub` instead.

### Repeat/Until

```
L0:
    <body>
    <condition>
    jz L0             ; repeat while false
```

### If/Then/Else

```
    <condition>
    jz L0             ; skip then-branch
    <then body>
    jmp L1            ; skip else-branch
L0:
    <else body>
L1:
```

### Case Statement

```
    <selector>
    dup               ; keep selector for each test
    push <value1>
    eq
    jz L0
    drop              ; matched: discard selector copy
    <body1>
    jmp Lexit
L0:
    dup
    push <value2>
    eq
    jz L1
    drop
    <body2>
    jmp Lexit
L1:
    drop              ; no match: discard selector
Lexit:
```

### Procedure Call

```
    <arg0 expr>       ; push arguments left-to-right
    <arg1 expr>
    call <proc_name>
```

### Function Call (with Return Value)

Same as procedure call; function pushes return value onto eval stack
before `ret`. Caller finds it on the eval stack after call returns.

### Boolean NOT

```
    <expr>
    push 0
    eq                ; 0 becomes 1, nonzero becomes 0
```

Not bitwise NOT; this is logical NOT.

### Var Parameter (Pass-by-Reference)

Caller pushes address:
```
    addrg <var>       ; or addrl/addra
    call <proc>
```

Callee dereferences:
```
    loada 0           ; get address
    load              ; dereference for read
    ; or
    loada 0           ; get address
    <new value>
    swap
    store             ; dereference for write
```

### Array Element Access

```
    addrg <array>     ; base address
    <index expr>
    push <lower_bound>
    sub               ; index - lower_bound
    push <elem_size>  ; 3 for word-sized elements
    mul
    add               ; base + (index - lo) * size
    load              ; dereference
```

### Record Field Access

```
    addrg <record>    ; base address
    push <field_offset_bytes>
    add               ; base + offset
    load              ; dereference
```

### String Literal

```
    push S0           ; address of .data block
    call _p24p_write_str
```

## Runtime Library Symbols

The compiler emits `.extern` declarations for these runtime routines:

| Symbol | Description |
|--------|-------------|
| `_p24p_write_int` | Write integer to output (1 arg) |
| `_p24p_write_bool` | Write boolean to output (1 arg) |
| `_p24p_write_str` | Write null-terminated string (1 arg: address) |
| `_p24p_write_ln` | Write newline (0 args) |
| `_p24p_read_int` | Read integer from input (0 args, pushes result) |
| `_p24p_read_ln` | Consume input line (0 args) |
| `_p24p_led_on` | Turn on LED D2 (0 args) |
| `_p24p_led_off` | Turn off LED D2 (0 args) |
