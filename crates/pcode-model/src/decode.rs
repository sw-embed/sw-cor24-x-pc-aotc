//! P-code binary file decoder.
//!
//! Reads a `.p24` binary (header + code + data) and produces a [`Program`]
//! with fully decoded instructions and procedure metadata.

use crate::{
    Encoding, Instruction, Opcode, Operand, ProcedureInfo, Program, P24_HEADER_SIZE, P24_MAGIC,
    P24_VERSION,
};

/// Errors that can occur while decoding a `.p24` binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// File is too short to contain a valid header.
    TooShort,
    /// Magic bytes don't match "P24\0".
    BadMagic,
    /// Unsupported version byte.
    BadVersion(u8),
    /// Body is shorter than declared code_size + data_size.
    Truncated,
    /// Invalid opcode byte encountered during instruction decoding.
    InvalidOpcode { pc: u32, byte: u8 },
    /// Instruction operand extends past end of code segment.
    UnexpectedEnd { pc: u32 },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::TooShort => write!(f, "file too short for .p24 header"),
            DecodeError::BadMagic => write!(f, "invalid .p24 magic bytes"),
            DecodeError::BadVersion(v) => write!(f, "unsupported .p24 version: {v}"),
            DecodeError::Truncated => write!(f, "file body truncated"),
            DecodeError::InvalidOpcode { pc, byte } => {
                write!(f, "invalid opcode 0x{byte:02X} at PC=0x{pc:04X}")
            }
            DecodeError::UnexpectedEnd { pc } => {
                write!(f, "instruction at PC=0x{pc:04X} extends past end of code")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

fn read_le24(bytes: &[u8]) -> u32 {
    bytes[0] as u32 | ((bytes[1] as u32) << 8) | ((bytes[2] as u32) << 16)
}

/// Decode a `.p24` binary into a [`Program`].
pub fn decode_program(binary: &[u8]) -> Result<Program, DecodeError> {
    // ── Parse header ────────────────────────────────────────────────
    if binary.len() < P24_HEADER_SIZE {
        return Err(DecodeError::TooShort);
    }
    if binary[0..4] != P24_MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = binary[4];
    if version != P24_VERSION {
        return Err(DecodeError::BadVersion(version));
    }

    let entry_point = read_le24(&binary[5..8]);
    let code_size = read_le24(&binary[8..11]) as usize;
    let data_size = read_le24(&binary[11..14]) as usize;
    let global_count = read_le24(&binary[14..17]);
    // binary[17] is flags, reserved

    let body = &binary[P24_HEADER_SIZE..];
    if body.len() < code_size + data_size {
        return Err(DecodeError::Truncated);
    }

    let code = &body[..code_size];
    let data = body[code_size..code_size + data_size].to_vec();

    // ── Decode instructions ─────────────────────────────────────────
    let instructions = decode_instructions(code)?;

    // ── Extract procedure metadata ──────────────────────────────────
    let procedures = extract_procedures(&instructions);

    Ok(Program {
        entry_point,
        instructions,
        data,
        global_count,
        procedures,
    })
}

/// Decode all instructions from a code segment.
fn decode_instructions(code: &[u8]) -> Result<Vec<Instruction>, DecodeError> {
    let mut instructions = Vec::new();
    let mut pc: usize = 0;

    while pc < code.len() {
        let op_byte = code[pc];
        let op = Opcode::from_byte(op_byte).ok_or(DecodeError::InvalidOpcode {
            pc: pc as u32,
            byte: op_byte,
        })?;

        let instr_size = op.size();
        if pc + instr_size > code.len() {
            return Err(DecodeError::UnexpectedEnd { pc: pc as u32 });
        }

        let operand = match op.encoding() {
            Encoding::None => Operand::None,
            Encoding::Imm8 => Operand::Imm8(code[pc + 1]),
            Encoding::Imm24 => Operand::Imm24(read_le24(&code[pc + 1..pc + 4])),
            Encoding::D8A24 => Operand::D8A24 {
                depth: code[pc + 1],
                addr: read_le24(&code[pc + 2..pc + 5]),
            },
            Encoding::D8O8 => Operand::D8O8 {
                depth: code[pc + 1],
                offset: code[pc + 2],
            },
        };

        instructions.push(Instruction {
            pc: pc as u32,
            op,
            operand,
        });

        pc += instr_size;
    }

    Ok(instructions)
}

/// Scan the instruction stream for `enter` instructions to identify procedures.
///
/// Each procedure starts at the instruction that is the target of a `call`/`calln`,
/// but since we don't resolve call targets here, we use `enter` as the marker.
/// The procedure's `entry_pc` is set to the PC of the `enter` instruction.
/// The procedure extends until the next `enter` or end of instructions.
fn extract_procedures(instructions: &[Instruction]) -> Vec<ProcedureInfo> {
    let mut procedures = Vec::new();

    for (idx, instr) in instructions.iter().enumerate() {
        if instr.op == Opcode::Enter {
            let num_locals = match instr.operand {
                Operand::Imm8(n) => n,
                _ => 0,
            };

            // Close previous procedure if any
            if let Some(prev) = procedures.last_mut() {
                let prev: &mut ProcedureInfo = prev;
                if prev.instr_end == 0 {
                    prev.instr_end = idx;
                }
            }

            procedures.push(ProcedureInfo {
                entry_pc: instr.pc,
                num_locals,
                instr_start: idx,
                instr_end: 0, // will be filled in
            });
        }
    }

    // Close last procedure
    if let Some(last) = procedures.last_mut() {
        if last.instr_end == 0 {
            last.instr_end = instructions.len();
        }
    }

    procedures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Opcode;

    /// Build a minimal .p24 binary for testing.
    fn make_p24(entry: u32, code: &[u8], data: &[u8], globals: u32) -> Vec<u8> {
        let mut binary = Vec::with_capacity(P24_HEADER_SIZE + code.len() + data.len());
        binary.extend_from_slice(&P24_MAGIC);
        binary.push(P24_VERSION);
        // entry point LE24
        binary.push(entry as u8);
        binary.push((entry >> 8) as u8);
        binary.push((entry >> 16) as u8);
        // code size LE24
        let cs = code.len() as u32;
        binary.push(cs as u8);
        binary.push((cs >> 8) as u8);
        binary.push((cs >> 16) as u8);
        // data size LE24
        let ds = data.len() as u32;
        binary.push(ds as u8);
        binary.push((ds >> 8) as u8);
        binary.push((ds >> 16) as u8);
        // global count LE24
        binary.push(globals as u8);
        binary.push((globals >> 8) as u8);
        binary.push((globals >> 16) as u8);
        // flags
        binary.push(0x00);
        binary.extend_from_slice(code);
        binary.extend_from_slice(data);
        binary
    }

    #[test]
    fn decode_empty_code() {
        let p24 = make_p24(0, &[], &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert!(prog.instructions.is_empty());
        assert!(prog.procedures.is_empty());
    }

    #[test]
    fn decode_halt() {
        let p24 = make_p24(0, &[0x00], &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.instructions.len(), 1);
        assert_eq!(prog.instructions[0].op, Opcode::Halt);
        assert_eq!(prog.instructions[0].operand, Operand::None);
        assert_eq!(prog.instructions[0].pc, 0);
    }

    #[test]
    fn decode_push_imm24() {
        // push 42 = 0x01, 0x2A, 0x00, 0x00
        let p24 = make_p24(0, &[0x01, 0x2A, 0x00, 0x00, 0x00], &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.instructions.len(), 2); // push + halt
        assert_eq!(prog.instructions[0].op, Opcode::Push);
        assert_eq!(prog.instructions[0].operand, Operand::Imm24(42));
    }

    #[test]
    fn decode_push_s() {
        // push_s 5 = 0x02, 0x05
        let p24 = make_p24(0, &[0x02, 0x05, 0x00], &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.instructions[0].op, Opcode::PushS);
        assert_eq!(prog.instructions[0].operand, Operand::Imm8(5));
    }

    #[test]
    fn decode_sequence() {
        // enter 2, push_s 10, push_s 3, add, storel 0, leave, ret 0, halt
        let code = &[
            0x40, 0x02, // enter 2
            0x02, 0x0A, // push_s 10
            0x02, 0x03, // push_s 3
            0x10, // add
            0x43, 0x00, // storel 0
            0x41, // leave
            0x34, 0x00, // ret 0
            0x00, // halt
        ];
        let p24 = make_p24(0, code, &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.instructions.len(), 8);

        assert_eq!(prog.instructions[0].op, Opcode::Enter);
        assert_eq!(prog.instructions[0].operand, Operand::Imm8(2));

        assert_eq!(prog.instructions[1].op, Opcode::PushS);
        assert_eq!(prog.instructions[1].operand, Operand::Imm8(10));

        assert_eq!(prog.instructions[3].op, Opcode::Add);
        assert_eq!(prog.instructions[3].operand, Operand::None);

        assert_eq!(prog.instructions[6].op, Opcode::Ret);
        assert_eq!(prog.instructions[6].operand, Operand::Imm8(0));
    }

    #[test]
    fn decode_calln() {
        // calln depth=2 addr=0x000100
        let code = &[0x35, 0x02, 0x00, 0x01, 0x00, 0x00];
        let p24 = make_p24(0, code, &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.instructions[0].op, Opcode::Calln);
        assert_eq!(
            prog.instructions[0].operand,
            Operand::D8A24 {
                depth: 2,
                addr: 0x000100
            }
        );
    }

    #[test]
    fn decode_loadn() {
        // loadn depth=1 offset=3
        let code = &[0x4A, 0x01, 0x03, 0x00];
        let p24 = make_p24(0, code, &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.instructions[0].op, Opcode::Loadn);
        assert_eq!(
            prog.instructions[0].operand,
            Operand::D8O8 {
                depth: 1,
                offset: 3
            }
        );
    }

    #[test]
    fn decode_data_segment() {
        let data = b"Hello\0";
        let p24 = make_p24(0, &[0x00], data, 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.data, data);
    }

    #[test]
    fn decode_global_count() {
        let p24 = make_p24(0, &[0x00], &[], 5);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.global_count, 5);
    }

    #[test]
    fn decode_entry_point() {
        let p24 = make_p24(0x10, &[0x00; 0x11], &[], 0);
        let prog = decode_program(&p24).unwrap();
        assert_eq!(prog.entry_point, 0x10);
    }

    #[test]
    fn procedure_extraction() {
        // Two procedures: first at PC=0, second at PC=4
        let code = &[
            0x40, 0x01, // enter 1 (proc 0)
            0x41, // leave
            0x34, 0x00, // ret 0
            0x40, 0x03, // enter 3 (proc 1)
            0x41, // leave
            0x34, 0x00, // ret 0
        ];
        let p24 = make_p24(0, code, &[], 0);
        let prog = decode_program(&p24).unwrap();

        assert_eq!(prog.procedures.len(), 2);
        assert_eq!(prog.procedures[0].entry_pc, 0);
        assert_eq!(prog.procedures[0].num_locals, 1);
        assert_eq!(prog.procedures[0].instr_start, 0);
        assert_eq!(prog.procedures[0].instr_end, 3); // up to (not including) proc 1's enter

        assert_eq!(prog.procedures[1].entry_pc, 5);
        assert_eq!(prog.procedures[1].num_locals, 3);
        assert_eq!(prog.procedures[1].instr_start, 3);
        assert_eq!(prog.procedures[1].instr_end, 6);
    }

    #[test]
    fn error_too_short() {
        assert_eq!(
            decode_program(&[0x50, 0x32]).unwrap_err(),
            DecodeError::TooShort
        );
    }

    #[test]
    fn error_bad_magic() {
        let mut p24 = make_p24(0, &[0x00], &[], 0);
        p24[0] = 0xFF;
        assert_eq!(decode_program(&p24).unwrap_err(), DecodeError::BadMagic);
    }

    #[test]
    fn error_bad_version() {
        let mut p24 = make_p24(0, &[0x00], &[], 0);
        p24[4] = 0x02;
        assert_eq!(
            decode_program(&p24).unwrap_err(),
            DecodeError::BadVersion(2)
        );
    }

    #[test]
    fn error_truncated() {
        let mut p24 = make_p24(0, &[0x00], &[], 0);
        // Claim code_size = 100 but only provide 1 byte
        p24[8] = 100;
        assert_eq!(decode_program(&p24).unwrap_err(), DecodeError::Truncated);
    }

    #[test]
    fn error_invalid_opcode() {
        let p24 = make_p24(0, &[0x07], &[], 0);
        assert_eq!(
            decode_program(&p24).unwrap_err(),
            DecodeError::InvalidOpcode { pc: 0, byte: 0x07 }
        );
    }

    #[test]
    fn error_unexpected_end() {
        // push requires 4 bytes but only 2 remain
        let p24 = make_p24(0, &[0x01, 0x00], &[], 0);
        assert_eq!(
            decode_program(&p24).unwrap_err(),
            DecodeError::UnexpectedEnd { pc: 0 }
        );
    }

    #[test]
    fn instruction_display() {
        let instr = Instruction {
            pc: 0x0010,
            op: Opcode::Push,
            operand: Operand::Imm24(42),
        };
        assert_eq!(format!("{instr}"), "0010  push 0x00002A");

        let instr2 = Instruction {
            pc: 0,
            op: Opcode::Halt,
            operand: Operand::None,
        };
        assert_eq!(format!("{instr2}"), "0000  halt");
    }

    #[test]
    fn pc_tracking() {
        // Verify that PCs are correctly tracked through variable-size instructions
        let code = &[
            0x01, 0x0A, 0x00, 0x00, // push 10    (4 bytes, PC=0)
            0x02, 0x05, // push_s 5   (2 bytes, PC=4)
            0x10, // add        (1 byte,  PC=6)
            0x00, // halt       (1 byte,  PC=7)
        ];
        let p24 = make_p24(0, code, &[], 0);
        let prog = decode_program(&p24).unwrap();

        assert_eq!(prog.instructions[0].pc, 0);
        assert_eq!(prog.instructions[1].pc, 4);
        assert_eq!(prog.instructions[2].pc, 6);
        assert_eq!(prog.instructions[3].pc, 7);
    }
}
