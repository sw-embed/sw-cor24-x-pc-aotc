//! Integration tests that decode real .p24 files compiled from Pascal test programs.

use pcode_model::{decode_program, Opcode, Operand, Program};
use std::path::PathBuf;

fn test_p24_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/p24")
}

fn load_test_program(name: &str) -> Program {
    let path = test_p24_dir().join(format!("{name}.p24"));
    let binary =
        std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    decode_program(&binary).unwrap_or_else(|e| panic!("decode failed for {}: {e}", path.display()))
}

/// Verify that all test .p24 files decode without errors.
#[test]
fn all_test_files_decode() {
    let names = [
        "arithmetic",
        "locals",
        "if_else",
        "while_loop",
        "procedure_call",
        "recursion",
    ];
    for name in names {
        let prog = load_test_program(name);
        assert!(
            !prog.instructions.is_empty(),
            "{name}.p24: decoded zero instructions"
        );
        // Every program should have at least one procedure (main)
        assert!(
            !prog.procedures.is_empty(),
            "{name}.p24: no procedures found"
        );
    }
}

/// Verify arithmetic.p24 has expected structure.
/// The Pascal source sets a=10, b=3 and performs arithmetic.
#[test]
fn arithmetic_structure() {
    let prog = load_test_program("arithmetic");

    // Should have globals (a, b, c = 3 globals)
    assert!(
        prog.global_count >= 3,
        "expected >= 3 globals, got {}",
        prog.global_count
    );

    // First instruction of main should be enter
    let main_entry = prog.entry_point as u32;
    let entry_instr = prog
        .instructions
        .iter()
        .find(|i| i.pc == main_entry)
        .expect("no instruction at entry point");
    assert_eq!(entry_instr.op, Opcode::Enter);

    // Should contain storeg instructions (storing to globals a, b, c)
    let storeg_count = prog
        .instructions
        .iter()
        .filter(|i| i.op == Opcode::Storeg)
        .count();
    assert!(
        storeg_count >= 3,
        "expected storeg instructions for globals"
    );

    // Should contain call instructions (to write routines)
    let call_count = prog
        .instructions
        .iter()
        .filter(|i| i.op == Opcode::Call)
        .count();
    assert!(call_count > 0, "expected call instructions for write");

    // Should contain halt somewhere (may not be last due to linked runtime code)
    let has_halt = prog.instructions.iter().any(|i| i.op == Opcode::Halt);
    assert!(has_halt, "program should contain halt");
}

/// Verify locals.p24 uses local variable instructions.
#[test]
fn locals_uses_local_ops() {
    let prog = load_test_program("locals");

    let has_loadl = prog.instructions.iter().any(|i| i.op == Opcode::Loadl);
    let has_storel = prog.instructions.iter().any(|i| i.op == Opcode::Storel);
    assert!(has_loadl, "locals.p24 should use loadl");
    assert!(has_storel, "locals.p24 should use storel");
}

/// Verify if_else.p24 uses conditional jumps.
#[test]
fn if_else_has_branches() {
    let prog = load_test_program("if_else");

    let has_jz = prog.instructions.iter().any(|i| i.op == Opcode::Jz);
    let has_jmp = prog.instructions.iter().any(|i| i.op == Opcode::Jmp);
    assert!(
        has_jz || has_jmp,
        "if_else.p24 should have branch instructions"
    );

    // Should have comparison instructions
    let has_cmp = prog.instructions.iter().any(|i| {
        matches!(
            i.op,
            Opcode::Eq | Opcode::Ne | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge
        )
    });
    assert!(has_cmp, "if_else.p24 should have comparison instructions");
}

/// Verify while_loop.p24 has backward jumps (loop structure).
#[test]
fn while_loop_has_back_edges() {
    let prog = load_test_program("while_loop");

    // Should have jmp instructions, at least one jumping backward
    let has_backward_jmp = prog.instructions.iter().any(|i| {
        if let Operand::Imm24(target) = i.operand {
            (i.op == Opcode::Jmp || i.op == Opcode::Jnz) && target < i.pc
        } else {
            false
        }
    });
    assert!(
        has_backward_jmp,
        "while_loop.p24 should have backward jumps"
    );
}

/// Verify procedure_call.p24 uses call/ret and enter/leave.
#[test]
fn procedure_call_structure() {
    let prog = load_test_program("procedure_call");

    let has_call = prog.instructions.iter().any(|i| i.op == Opcode::Call);
    let has_ret = prog.instructions.iter().any(|i| i.op == Opcode::Ret);
    let has_enter = prog.instructions.iter().any(|i| i.op == Opcode::Enter);
    let has_leave = prog.instructions.iter().any(|i| i.op == Opcode::Leave);

    assert!(has_call, "should have call instructions");
    assert!(has_ret, "should have ret instructions");
    assert!(has_enter, "should have enter instructions");
    assert!(has_leave, "should have leave instructions");

    // Should have multiple procedures
    assert!(
        prog.procedures.len() >= 2,
        "expected >= 2 procedures, got {}",
        prog.procedures.len()
    );
}

/// Verify recursion.p24 structure (iterative factorial + fibonacci).
#[test]
fn recursion_structure() {
    let prog = load_test_program("recursion");

    // Should have globals for i, n, fact, a, b, temp
    assert!(prog.global_count >= 6, "expected >= 6 globals");

    // Should have backward jumps for the loops
    let has_backward_jmp = prog.instructions.iter().any(|i| {
        if let Operand::Imm24(target) = i.operand {
            i.op == Opcode::Jmp && target < i.pc
        } else {
            false
        }
    });
    assert!(has_backward_jmp, "should have backward jumps for loops");

    // Should have mul instruction for factorial computation
    let has_mul = prog.instructions.iter().any(|i| i.op == Opcode::Mul);
    assert!(has_mul, "should have mul for factorial computation");
}

/// Verify instruction PCs form a contiguous, non-overlapping sequence.
#[test]
fn instruction_pcs_are_contiguous() {
    let names = [
        "arithmetic",
        "locals",
        "if_else",
        "while_loop",
        "procedure_call",
        "recursion",
    ];
    for name in names {
        let prog = load_test_program(name);
        for window in prog.instructions.windows(2) {
            let expected_next = window[0].pc + window[0].size() as u32;
            assert_eq!(
                window[1].pc,
                expected_next,
                "{name}.p24: gap between PC=0x{:04X} (size {}) and PC=0x{:04X}",
                window[0].pc,
                window[0].size(),
                window[1].pc
            );
        }
    }
}

/// Print a disassembly of arithmetic.p24 for manual inspection.
#[test]
fn disassemble_arithmetic() {
    let prog = load_test_program("arithmetic");
    let mut disasm = String::new();
    for instr in &prog.instructions {
        disasm.push_str(&format!("{instr}\n"));
    }
    // Just verify it doesn't panic and produces output
    assert!(!disasm.is_empty());
    // Uncomment to see disassembly:
    // eprintln!("{disasm}");
}
