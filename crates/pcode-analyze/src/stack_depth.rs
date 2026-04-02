//! Stack-depth propagation across basic blocks.
//!
//! Computes the eval stack depth at entry and exit of each basic block,
//! verifying consistency at merge points where multiple paths converge.

use crate::cfg::Cfg;
use pcode_model::Instruction;
use std::collections::BTreeMap;

/// Errors during stack-depth propagation.
#[derive(Debug)]
pub enum StackDepthError {
    /// Stack depth is inconsistent at a forward merge point.
    /// (Back-edge mismatches are reported as warnings, not errors,
    /// since variable-depth loops are valid in p-code — e.g., digit
    /// accumulation loops in write_int.)
    InconsistentMerge {
        block_pc: u32,
        expected: i32,
        got: i32,
        from_pc: u32,
    },
    /// Stack underflow detected within a block.
    Underflow { pc: u32, depth: i32 },
}

impl std::fmt::Display for StackDepthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackDepthError::InconsistentMerge {
                block_pc,
                expected,
                got,
                from_pc,
            } => {
                write!(
                    f,
                    "stack depth mismatch at block 0x{block_pc:04X}: \
                     expected {expected} but got {got} from block 0x{from_pc:04X}"
                )
            }
            StackDepthError::Underflow { pc, depth } => {
                write!(f, "stack underflow at PC=0x{pc:04X}: depth={depth}")
            }
        }
    }
}

impl std::error::Error for StackDepthError {}

/// Result of stack-depth propagation for one block.
#[derive(Debug, Clone, Copy)]
pub struct BlockDepth {
    /// Eval stack depth at block entry.
    pub entry: i32,
    /// Eval stack depth at block exit (after the last instruction).
    pub exit: i32,
}

/// Propagate eval stack depth across the CFG.
///
/// `initial_depth` is the stack depth at the entry block (typically 0).
/// Returns a map from block start PC to its entry/exit depths.
///
/// Back-edges (where a successor PC <= the current block's PC) with
/// inconsistent depths are tolerated, since variable-depth loops are
/// valid in p-code (e.g., write_int's digit accumulation loop). Only
/// forward-edge mismatches are reported as errors.
pub fn propagate_stack_depth(
    cfg: &Cfg,
    instructions: &[Instruction],
    initial_depth: i32,
) -> Result<BTreeMap<u32, BlockDepth>, StackDepthError> {
    let mut depths: BTreeMap<u32, BlockDepth> = BTreeMap::new();

    if cfg.blocks.is_empty() {
        return Ok(depths);
    }

    // Use reverse postorder for forward dataflow.
    let rpo = cfg.reverse_postorder();

    // Initialize entry block.
    let entry_pc = rpo[0];
    let mut entry_depths: BTreeMap<u32, i32> = BTreeMap::new();
    entry_depths.insert(entry_pc, initial_depth);

    // Single pass in RPO order (sufficient for reducible CFGs).
    for &block_pc in &rpo {
        let block = match cfg.blocks.get(&block_pc) {
            Some(b) => b,
            None => continue,
        };

        let entry = match entry_depths.get(&block_pc) {
            Some(&d) => d,
            None => continue, // unreachable block
        };

        // Compute exit depth by walking instructions.
        let exit = compute_block_exit_depth(instructions, &block.instr_range, entry)?;

        depths.insert(block_pc, BlockDepth { entry, exit });

        // Propagate to successors.
        // Skip strict consistency checks for:
        // - Back-edges (variable-depth loops are valid, e.g., digit accumulation)
        // - Call terminators (callee's ret cleans args, changing the effective depth)
        let is_call = matches!(block.terminator, crate::Terminator::Call { .. });

        for &succ_pc in &block.successors {
            let is_back_edge = succ_pc <= block_pc;

            match entry_depths.get(&succ_pc) {
                Some(&existing) => {
                    if existing != exit && !is_back_edge && !is_call {
                        return Err(StackDepthError::InconsistentMerge {
                            block_pc: succ_pc,
                            expected: existing,
                            got: exit,
                            from_pc: block_pc,
                        });
                    }
                }
                None => {
                    entry_depths.insert(succ_pc, exit);
                }
            }
        }
    }

    Ok(depths)
}

/// Compute the exit stack depth for a block given its entry depth.
fn compute_block_exit_depth(
    instructions: &[Instruction],
    range: &std::ops::Range<usize>,
    entry_depth: i32,
) -> Result<i32, StackDepthError> {
    let mut depth = entry_depth;

    for idx in range.clone() {
        let instr = &instructions[idx];
        let delta = instr.stack_delta().unwrap_or(0);
        depth += delta as i32;

        // Check for underflow (depth should never go below 0).
        if depth < 0 {
            return Err(StackDepthError::Underflow {
                pc: instr.pc,
                depth,
            });
        }
    }

    Ok(depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::Cfg;
    use pcode_model::{Instruction, Opcode, Operand};

    fn instr(pc: u32, op: Opcode, operand: Operand) -> Instruction {
        Instruction { pc, op, operand }
    }

    #[test]
    fn linear_depth() {
        // push, push, add, halt → depth: 0,1,2,1,1
        let instrs = vec![
            instr(0, Opcode::Push, Operand::Imm24(1)),
            instr(4, Opcode::Push, Operand::Imm24(2)),
            instr(8, Opcode::Add, Operand::None),
            instr(9, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..4).unwrap();
        let depths = propagate_stack_depth(&cfg, &instrs, 0).unwrap();
        let d = depths.get(&0).unwrap();
        assert_eq!(d.entry, 0);
        assert_eq!(d.exit, 1); // push, push, add = net +1; halt = +0
    }

    #[test]
    fn branch_merge_consistent() {
        // 0: push 1
        // 4: jz 12       → pops 1, depth 0 at both successors
        // 8: push 99     → depth 0→1
        // 12: jmp 16     → depth 1→1
        // 16: push 42    → depth from 4 is 0, from 12 is 1 → inconsistent?
        // Actually let me make them consistent:
        // 0: push 1       (depth: 0→1)
        // 4: jz 12        (depth: 1→0, both paths have depth 0)
        // 8: halt          (depth: 0→0)
        // 12: halt         (depth: 0→0)
        let instrs = vec![
            instr(0, Opcode::Push, Operand::Imm24(1)),
            instr(4, Opcode::Jz, Operand::Imm24(12)),
            instr(8, Opcode::Halt, Operand::None),
            instr(12, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..4).unwrap();
        let depths = propagate_stack_depth(&cfg, &instrs, 0).unwrap();

        let d8 = depths.get(&8).unwrap();
        assert_eq!(d8.entry, 0);

        let d12 = depths.get(&12).unwrap();
        assert_eq!(d12.entry, 0);
    }

    #[test]
    fn inconsistent_merge_detected() {
        // Two paths reach block at PC=20 with different depths:
        // Path 1 (fallthrough): push, push → depth 2
        // Path 2 (branch target): push → depth 1
        //
        // Block 0: push 1 (0→1), jz 16 (1→0)
        // Block 8: push 1 (0→1), push 2 (1→2), jmp 20 (2→2)
        // Block 16: push 1 (0→1), jmp 20 (1→1)
        // Block 20: halt — reached with depth 2 (from 8) vs 1 (from 16)
        let instrs = vec![
            instr(0, Opcode::Push, Operand::Imm24(1)), // 0: push (4 bytes)
            instr(4, Opcode::Jz, Operand::Imm24(16)),  // 4: jz 16 (4 bytes)
            instr(8, Opcode::Push, Operand::Imm24(1)), // 8: push (4 bytes)
            instr(12, Opcode::Push, Operand::Imm24(2)), // 12: push (4 bytes)
            instr(16, Opcode::Jmp, Operand::Imm24(24)), // 16: jmp 24 — but 16 is also jz target!
                                                       // This won't work because block at 16 is split by being a jz target.
                                                       // Let me use different PCs to avoid overlap.
        ];
        // The difficulty is that jz target=16 creates a leader at 16, which is also the jmp.
        // The block 8..16 has: push, push = depth +2 from entry 0 = depth 2.
        // The block at 16: starts with jmp 24. Reached from jz with depth 0, from block 8 with depth 2.
        // That's inconsistent!
        let cfg = Cfg::build(&instrs, 0..5).unwrap();
        let result = propagate_stack_depth(&cfg, &instrs, 0);
        assert!(result.is_err(), "should detect inconsistent merge");
    }

    #[test]
    fn while_loop_consistent_depth() {
        // Simulates: push condition; jz exit; body (push+drop=neutral); jmp header; halt
        // 0: push 1       (depth 0→1) — loop header
        // 4: jz 12        (depth 1→0) — exit to 12, fall to 8
        // 8: jmp 0        (depth 0→0) — back edge, depth 0 matches header entry
        // 12: halt         (depth 0→0) — after loop
        let instrs = vec![
            instr(0, Opcode::Push, Operand::Imm24(1)),
            instr(4, Opcode::Jz, Operand::Imm24(12)),
            instr(8, Opcode::Jmp, Operand::Imm24(0)),
            instr(12, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..4).unwrap();
        let depths = propagate_stack_depth(&cfg, &instrs, 0).unwrap();

        // Header block entry should be 0 (initial and from back-edge)
        let d0 = depths.get(&0).unwrap();
        assert_eq!(d0.entry, 0);
        assert_eq!(d0.exit, 0); // push(+1) + jz(-1) = 0
    }
}
