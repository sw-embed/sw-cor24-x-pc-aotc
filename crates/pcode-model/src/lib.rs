//! pcode-model — P-code opcode definitions, decoded instructions, and binary file reader.
//!
//! This crate defines the COR24 p-code instruction set and provides a decoder
//! that reads `.p24` binary files into a structured [`Program`] representation.
//!
//! The opcode values and encoding formats match the canonical p-code VM (pvm.s)
//! and assembler (pa24r) exactly.

mod decode;

pub use decode::{decode_program, DecodeError};

// ── .p24 file format constants ──────────────────────────────────────

pub const P24_MAGIC: [u8; 4] = [0x50, 0x32, 0x34, 0x00]; // "P24\0"
pub const P24_VERSION: u8 = 1;
pub const P24_HEADER_SIZE: usize = 18;

/// Word size in bytes (COR24 is a 24-bit architecture).
pub const WORD: usize = 3;

// ── Encoding ────────────────────────────────────────────────────────

/// Instruction encoding format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// 1 byte: `[op]`
    None,
    /// 2 bytes: `[op, imm8]`
    Imm8,
    /// 4 bytes: `[op, lo, mid, hi]`
    Imm24,
    /// 5 bytes: `[op, d8, lo, mid, hi]`
    D8A24,
    /// 3 bytes: `[op, d8, o8]`
    D8O8,
}

impl Encoding {
    /// Total instruction size in bytes (including opcode byte).
    pub const fn size(self) -> usize {
        match self {
            Encoding::None => 1,
            Encoding::Imm8 => 2,
            Encoding::Imm24 => 4,
            Encoding::D8A24 => 5,
            Encoding::D8O8 => 3,
        }
    }
}

// ── Opcode ──────────────────────────────────────────────────────────

/// P-code opcodes. Values match pvm.s dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Opcode {
    // Stack operations (0x00–0x06)
    /// Stop execution. `( -- )`
    Halt = 0x00,
    /// Push 24-bit signed immediate. `( -- n )`
    Push = 0x01,
    /// Push sign-extended 8-bit immediate. `( -- n )`
    PushS = 0x02,
    /// Duplicate TOS. `( a -- a a )`
    Dup = 0x03,
    /// Discard TOS. `( a -- )`
    Drop = 0x04,
    /// Swap TOS and NOS. `( a b -- b a )`
    Swap = 0x05,
    /// Copy NOS over TOS. `( a b -- a b a )`
    Over = 0x06,

    // Arithmetic (0x10–0x15)
    /// `( a b -- a+b )`
    Add = 0x10,
    /// `( a b -- a-b )`
    Sub = 0x11,
    /// `( a b -- a*b )`
    Mul = 0x12,
    /// `( a b -- a/b )` — traps on divide by zero
    Div = 0x13,
    /// `( a b -- a%b )` — traps on divide by zero
    Mod = 0x14,
    /// `( a -- -a )`
    Neg = 0x15,

    // Logic / bitwise (0x16–0x1B)
    /// `( a b -- a&b )`
    And = 0x16,
    /// `( a b -- a|b )`
    Or = 0x17,
    /// `( a b -- a^b )`
    Xor = 0x18,
    /// `( a -- ~a )`
    Not = 0x19,
    /// `( a n -- a<<n )`
    Shl = 0x1A,
    /// `( a n -- a>>n )` — arithmetic shift right
    Shr = 0x1B,

    // Comparison (0x20–0x25) — all push 1 (true) or 0 (false)
    /// `( a b -- flag )`
    Eq = 0x20,
    /// `( a b -- flag )`
    Ne = 0x21,
    /// `( a b -- flag )` — signed
    Lt = 0x22,
    /// `( a b -- flag )` — signed
    Le = 0x23,
    /// `( a b -- flag )` — signed
    Gt = 0x24,
    /// `( a b -- flag )` — signed
    Ge = 0x25,

    // Control flow (0x30–0x36)
    /// Unconditional jump. `( -- )`
    Jmp = 0x30,
    /// Jump if TOS == 0. `( flag -- )`
    Jz = 0x31,
    /// Jump if TOS != 0. `( flag -- )`
    Jnz = 0x32,
    /// Call procedure (flat, static link = dynamic link). `( args... -- )`
    Call = 0x33,
    /// Return from procedure, clean nargs. `( [rv] -- [rv] )`
    Ret = 0x34,
    /// Call with explicit static link depth. `( args... -- )`
    Calln = 0x35,
    /// Trigger trap with code. `( -- )`
    Trap = 0x36,

    // Frame / local / global / nonlocal access (0x40–0x4B)
    /// Set up frame, allocate nlocals slots. `( -- )`
    Enter = 0x40,
    /// Tear down frame (deallocate locals). `( -- )`
    Leave = 0x41,
    /// Load local variable at offset. `( -- val )`
    Loadl = 0x42,
    /// Store to local variable at offset. `( val -- )`
    Storel = 0x43,
    /// Load global variable at offset. `( -- val )`
    Loadg = 0x44,
    /// Store to global variable at offset. `( val -- )`
    Storeg = 0x45,
    /// Push address of local at offset. `( -- addr )`
    Addrl = 0x46,
    /// Push address of global at offset. `( -- addr )`
    Addrg = 0x47,
    /// Load argument at index. `( -- val )`
    Loada = 0x48,
    /// Store to argument at index. `( val -- )`
    Storea = 0x49,
    /// Load nonlocal (depth, offset). `( -- val )`
    Loadn = 0x4A,
    /// Store nonlocal (depth, offset). `( val -- )`
    Storen = 0x4B,

    // Indirect memory (0x50–0x53)
    /// Load word from address. `( addr -- val )` — traps on nil
    Load = 0x50,
    /// Store word to address. `( val addr -- )` — traps on nil
    Store = 0x51,
    /// Load byte (zero-extended) from address. `( addr -- byte )` — traps on nil
    Loadb = 0x52,
    /// Store byte to address. `( byte addr -- )` — traps on nil
    Storeb = 0x53,

    // System calls (0x60)
    /// System call by id. Stack effect varies by id.
    Sys = 0x60,
}

impl Opcode {
    /// Return the encoding format for this opcode.
    pub const fn encoding(self) -> Encoding {
        match self {
            Opcode::Halt
            | Opcode::Dup
            | Opcode::Drop
            | Opcode::Swap
            | Opcode::Over
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Neg
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Not
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge
            | Opcode::Leave
            | Opcode::Load
            | Opcode::Store
            | Opcode::Loadb
            | Opcode::Storeb => Encoding::None,

            Opcode::PushS
            | Opcode::Ret
            | Opcode::Trap
            | Opcode::Enter
            | Opcode::Loadl
            | Opcode::Storel
            | Opcode::Addrl
            | Opcode::Loada
            | Opcode::Storea
            | Opcode::Sys => Encoding::Imm8,

            Opcode::Push
            | Opcode::Jmp
            | Opcode::Jz
            | Opcode::Jnz
            | Opcode::Call
            | Opcode::Loadg
            | Opcode::Storeg
            | Opcode::Addrg => Encoding::Imm24,

            Opcode::Calln => Encoding::D8A24,

            Opcode::Loadn | Opcode::Storen => Encoding::D8O8,
        }
    }

    /// Instruction size in bytes.
    pub const fn size(self) -> usize {
        self.encoding().size()
    }

    /// Decode an opcode from its byte value.
    pub fn from_byte(byte: u8) -> Option<Opcode> {
        match byte {
            0x00 => Some(Opcode::Halt),
            0x01 => Some(Opcode::Push),
            0x02 => Some(Opcode::PushS),
            0x03 => Some(Opcode::Dup),
            0x04 => Some(Opcode::Drop),
            0x05 => Some(Opcode::Swap),
            0x06 => Some(Opcode::Over),
            0x10 => Some(Opcode::Add),
            0x11 => Some(Opcode::Sub),
            0x12 => Some(Opcode::Mul),
            0x13 => Some(Opcode::Div),
            0x14 => Some(Opcode::Mod),
            0x15 => Some(Opcode::Neg),
            0x16 => Some(Opcode::And),
            0x17 => Some(Opcode::Or),
            0x18 => Some(Opcode::Xor),
            0x19 => Some(Opcode::Not),
            0x1A => Some(Opcode::Shl),
            0x1B => Some(Opcode::Shr),
            0x20 => Some(Opcode::Eq),
            0x21 => Some(Opcode::Ne),
            0x22 => Some(Opcode::Lt),
            0x23 => Some(Opcode::Le),
            0x24 => Some(Opcode::Gt),
            0x25 => Some(Opcode::Ge),
            0x30 => Some(Opcode::Jmp),
            0x31 => Some(Opcode::Jz),
            0x32 => Some(Opcode::Jnz),
            0x33 => Some(Opcode::Call),
            0x34 => Some(Opcode::Ret),
            0x35 => Some(Opcode::Calln),
            0x36 => Some(Opcode::Trap),
            0x40 => Some(Opcode::Enter),
            0x41 => Some(Opcode::Leave),
            0x42 => Some(Opcode::Loadl),
            0x43 => Some(Opcode::Storel),
            0x44 => Some(Opcode::Loadg),
            0x45 => Some(Opcode::Storeg),
            0x46 => Some(Opcode::Addrl),
            0x47 => Some(Opcode::Addrg),
            0x48 => Some(Opcode::Loada),
            0x49 => Some(Opcode::Storea),
            0x4A => Some(Opcode::Loadn),
            0x4B => Some(Opcode::Storen),
            0x50 => Some(Opcode::Load),
            0x51 => Some(Opcode::Store),
            0x52 => Some(Opcode::Loadb),
            0x53 => Some(Opcode::Storeb),
            0x60 => Some(Opcode::Sys),
            _ => None,
        }
    }

    /// Mnemonic string for this opcode.
    pub const fn mnemonic(self) -> &'static str {
        match self {
            Opcode::Halt => "halt",
            Opcode::Push => "push",
            Opcode::PushS => "push_s",
            Opcode::Dup => "dup",
            Opcode::Drop => "drop",
            Opcode::Swap => "swap",
            Opcode::Over => "over",
            Opcode::Add => "add",
            Opcode::Sub => "sub",
            Opcode::Mul => "mul",
            Opcode::Div => "div",
            Opcode::Mod => "mod",
            Opcode::Neg => "neg",
            Opcode::And => "and",
            Opcode::Or => "or",
            Opcode::Xor => "xor",
            Opcode::Not => "not",
            Opcode::Shl => "shl",
            Opcode::Shr => "shr",
            Opcode::Eq => "eq",
            Opcode::Ne => "ne",
            Opcode::Lt => "lt",
            Opcode::Le => "le",
            Opcode::Gt => "gt",
            Opcode::Ge => "ge",
            Opcode::Jmp => "jmp",
            Opcode::Jz => "jz",
            Opcode::Jnz => "jnz",
            Opcode::Call => "call",
            Opcode::Ret => "ret",
            Opcode::Calln => "calln",
            Opcode::Trap => "trap",
            Opcode::Enter => "enter",
            Opcode::Leave => "leave",
            Opcode::Loadl => "loadl",
            Opcode::Storel => "storel",
            Opcode::Loadg => "loadg",
            Opcode::Storeg => "storeg",
            Opcode::Addrl => "addrl",
            Opcode::Addrg => "addrg",
            Opcode::Loada => "loada",
            Opcode::Storea => "storea",
            Opcode::Loadn => "loadn",
            Opcode::Storen => "storen",
            Opcode::Load => "load",
            Opcode::Store => "store",
            Opcode::Loadb => "loadb",
            Opcode::Storeb => "storeb",
            Opcode::Sys => "sys",
        }
    }
}

impl Opcode {
    /// Return the eval stack depth change for this opcode.
    ///
    /// Positive means net pushes, negative means net pops.
    /// Returns `None` for opcodes whose effect is dynamic or context-dependent
    /// (e.g., `Ret`, `Sys`, `Enter`, `Leave`).
    pub fn stack_delta(self) -> Option<i8> {
        match self {
            // ( -- )
            Opcode::Halt | Opcode::Jmp | Opcode::Trap => Some(0),
            // ( -- n )
            Opcode::Push
            | Opcode::PushS
            | Opcode::Loadl
            | Opcode::Loadg
            | Opcode::Loada
            | Opcode::Addrl
            | Opcode::Addrg
            | Opcode::Loadn => Some(1),
            // ( a -- a a )
            Opcode::Dup => Some(1),
            // ( a -- )
            Opcode::Drop | Opcode::Storel | Opcode::Storeg | Opcode::Storea | Opcode::Storen => {
                Some(-1)
            }
            // ( a b -- b a )
            Opcode::Swap => Some(0),
            // ( a b -- a b a )
            Opcode::Over => Some(1),
            // ( a b -- c ) — binary ops
            Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge => Some(-1),
            // ( a -- b ) — unary ops
            Opcode::Neg | Opcode::Not => Some(0),
            // ( flag -- ) — conditional jump pops flag
            Opcode::Jz | Opcode::Jnz => Some(-1),
            // ( addr -- val )
            Opcode::Load | Opcode::Loadb => Some(0),
            // ( val addr -- )
            Opcode::Store | Opcode::Storeb => Some(-2),
            // Dynamic/context-dependent
            Opcode::Call
            | Opcode::Calln
            | Opcode::Ret
            | Opcode::Enter
            | Opcode::Leave
            | Opcode::Sys => None,
        }
    }
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.mnemonic())
    }
}

// ── System call IDs ─────────────────────────────────────────────────

/// Known system call identifiers (operand to `sys`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SysCall {
    Halt = 0,
    Putc = 1,
    Getc = 2,
    Led = 3,
    Alloc = 4,
    Free = 5,
    ReadSwitch = 6,
}

impl SysCall {
    pub fn from_id(id: u8) -> Option<SysCall> {
        match id {
            0 => Some(SysCall::Halt),
            1 => Some(SysCall::Putc),
            2 => Some(SysCall::Getc),
            3 => Some(SysCall::Led),
            4 => Some(SysCall::Alloc),
            5 => Some(SysCall::Free),
            6 => Some(SysCall::ReadSwitch),
            _ => None,
        }
    }
}

// ── Instruction ─────────────────────────────────────────────────────

/// Decoded operand payload, determined by the instruction's encoding format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// No operand (Encoding::None).
    None,
    /// 8-bit unsigned operand (Encoding::Imm8).
    /// For `push_s`, the value is sign-extended at execution time.
    Imm8(u8),
    /// 24-bit operand (Encoding::Imm24).
    /// For `push`, this is a signed constant (sign-extend the 24-bit value).
    /// For jumps/calls, this is an absolute code address.
    /// For loadg/storeg/addrg, this is a global word offset.
    Imm24(u32),
    /// Depth + address operand (Encoding::D8A24, used by `calln`).
    D8A24 { depth: u8, addr: u32 },
    /// Depth + offset operand (Encoding::D8O8, used by `loadn`/`storen`).
    D8O8 { depth: u8, offset: u8 },
}

/// A single decoded p-code instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction {
    /// Byte offset of this instruction within the code segment.
    pub pc: u32,
    /// The opcode.
    pub op: Opcode,
    /// Decoded operand.
    pub operand: Operand,
}

impl Instruction {
    /// Size of this instruction in bytes.
    pub fn size(&self) -> usize {
        self.op.size()
    }

    /// Return the eval stack depth change for this instruction.
    ///
    /// Unlike `Opcode::stack_delta`, this considers the operand for
    /// context-dependent opcodes like `Sys`. Returns `None` for opcodes
    /// whose effect truly cannot be determined statically (Call, Ret, Enter, Leave).
    pub fn stack_delta(&self) -> Option<i8> {
        match self.op {
            Opcode::Sys => {
                if let Operand::Imm8(id) = self.operand {
                    match SysCall::from_id(id) {
                        Some(SysCall::Halt) => Some(0),
                        Some(SysCall::Putc) => Some(-1), // ( char -- )
                        Some(SysCall::Getc) => Some(1),  // ( -- char )
                        Some(SysCall::Led) => Some(-1),  // ( val -- )
                        Some(SysCall::Alloc) => Some(0), // ( size -- addr )
                        Some(SysCall::Free) => Some(-1), // ( addr -- )
                        Some(SysCall::ReadSwitch) => Some(1), // ( -- val )
                        None => None,
                    }
                } else {
                    None
                }
            }
            _ => self.op.stack_delta(),
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}  {}", self.pc, self.op)?;
        match self.operand {
            Operand::None => {}
            Operand::Imm8(v) => write!(f, " {v}")?,
            Operand::Imm24(v) => write!(f, " 0x{v:06X}")?,
            Operand::D8A24 { depth, addr } => write!(f, " {depth} 0x{addr:06X}")?,
            Operand::D8O8 { depth, offset } => write!(f, " {depth} {offset}")?,
        }
        Ok(())
    }
}

// ── Program ─────────────────────────────────────────────────────────

/// Metadata about a procedure, derived from the instruction stream.
///
/// Procedures are identified by scanning for `enter` instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureInfo {
    /// Byte offset of this procedure's first instruction in the code segment.
    pub entry_pc: u32,
    /// Number of local variable slots (from the `enter` instruction).
    pub num_locals: u8,
    /// Number of arguments cleaned by `ret` (`None` if no `ret` found).
    pub nargs: Option<u8>,
    /// Index range into `Program::instructions` for this procedure's instructions.
    pub instr_start: usize,
    pub instr_end: usize,
}

/// A fully decoded p-code program.
#[derive(Debug, Clone)]
pub struct Program {
    /// Entry point (byte offset into code segment).
    pub entry_point: u32,
    /// All decoded instructions, in order of code offset.
    pub instructions: Vec<Instruction>,
    /// Raw data segment bytes.
    pub data: Vec<u8>,
    /// Number of global variable words.
    pub global_count: u32,
    /// Procedures discovered by scanning for `enter` instructions.
    pub procedures: Vec<ProcedureInfo>,
}

// ── Trap codes ──────────────────────────────────────────────────────

/// Trap codes matching pvm.s behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrapCode {
    UserTrap = 0,
    DivZero = 1,
    StackOverflow = 2,
    StackUnderflow = 3,
    InvalidOpcode = 4,
    InvalidAddress = 5,
    NilPointer = 6,
    BoundsCheck = 7,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_byte_roundtrip() {
        let all = [
            Opcode::Halt,
            Opcode::Push,
            Opcode::PushS,
            Opcode::Dup,
            Opcode::Drop,
            Opcode::Swap,
            Opcode::Over,
            Opcode::Add,
            Opcode::Sub,
            Opcode::Mul,
            Opcode::Div,
            Opcode::Mod,
            Opcode::Neg,
            Opcode::And,
            Opcode::Or,
            Opcode::Xor,
            Opcode::Not,
            Opcode::Shl,
            Opcode::Shr,
            Opcode::Eq,
            Opcode::Ne,
            Opcode::Lt,
            Opcode::Le,
            Opcode::Gt,
            Opcode::Ge,
            Opcode::Jmp,
            Opcode::Jz,
            Opcode::Jnz,
            Opcode::Call,
            Opcode::Ret,
            Opcode::Calln,
            Opcode::Trap,
            Opcode::Enter,
            Opcode::Leave,
            Opcode::Loadl,
            Opcode::Storel,
            Opcode::Loadg,
            Opcode::Storeg,
            Opcode::Addrl,
            Opcode::Addrg,
            Opcode::Loada,
            Opcode::Storea,
            Opcode::Loadn,
            Opcode::Storen,
            Opcode::Load,
            Opcode::Store,
            Opcode::Loadb,
            Opcode::Storeb,
            Opcode::Sys,
        ];
        for op in all {
            let byte = op as u8;
            let decoded = Opcode::from_byte(byte).unwrap_or_else(|| {
                panic!("from_byte failed for {:?} (0x{:02X})", op, byte);
            });
            assert_eq!(decoded, op);
        }
    }

    #[test]
    fn invalid_opcode_bytes() {
        assert_eq!(Opcode::from_byte(0x07), None);
        assert_eq!(Opcode::from_byte(0x0F), None);
        assert_eq!(Opcode::from_byte(0xFF), None);
    }

    #[test]
    fn encoding_sizes() {
        assert_eq!(Opcode::Halt.size(), 1);
        assert_eq!(Opcode::PushS.size(), 2);
        assert_eq!(Opcode::Push.size(), 4);
        assert_eq!(Opcode::Calln.size(), 5);
        assert_eq!(Opcode::Loadn.size(), 3);
    }
}
