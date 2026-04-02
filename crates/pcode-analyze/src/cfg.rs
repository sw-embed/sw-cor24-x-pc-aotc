//! Control flow graph construction from decoded p-code instructions.

use pcode_model::{Instruction, Opcode, Operand, Program};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Errors during CFG construction.
#[derive(Debug)]
pub enum CfgError {
    /// A jump target does not correspond to any instruction.
    InvalidTarget { from_pc: u32, target_pc: u32 },
}

impl std::fmt::Display for CfgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgError::InvalidTarget { from_pc, target_pc } => {
                write!(
                    f,
                    "jump at PC=0x{from_pc:04X} targets invalid address 0x{target_pc:04X}"
                )
            }
        }
    }
}

impl std::error::Error for CfgError {}

/// How a basic block ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    /// Falls through to the next block.
    Fallthrough,
    /// Unconditional jump to target PC.
    Jump(u32),
    /// Conditional branch: if condition met go to `target`, else fall through to `fallthrough`.
    Branch { target: u32, fallthrough: u32 },
    /// Return from procedure.
    Return,
    /// Halt (program termination).
    Halt,
    /// Procedure call: calls `target`, then falls through to `fallthrough`.
    Call { target: u32, fallthrough: u32 },
}

/// A basic block: a maximal sequence of instructions with no branches
/// in (except at the entry) and no branches out (except at the exit).
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// The p-code PC of the first instruction in this block.
    pub start_pc: u32,
    /// Instruction indices (into `Program::instructions`) in this block.
    pub instr_range: std::ops::Range<usize>,
    /// How this block ends.
    pub terminator: Terminator,
    /// Successor block PCs.
    pub successors: Vec<u32>,
    /// Predecessor block PCs.
    pub predecessors: Vec<u32>,
}

/// A control flow graph for a single procedure (or the whole program).
#[derive(Debug)]
pub struct Cfg {
    /// Basic blocks keyed by their start PC, in address order.
    pub blocks: BTreeMap<u32, BasicBlock>,
}

impl Cfg {
    /// Build a CFG from a slice of instructions (typically one procedure's worth).
    ///
    /// `instructions` should be the full `Program::instructions` slice.
    /// `range` specifies which indices to analyze (e.g., one procedure).
    pub fn build(
        instructions: &[Instruction],
        range: std::ops::Range<usize>,
    ) -> Result<Self, CfgError> {
        if range.is_empty() {
            return Ok(Cfg {
                blocks: BTreeMap::new(),
            });
        }

        // Step 1: Identify leaders (block start PCs).
        let leaders = find_leaders(instructions, &range);

        // Step 2: Partition instructions into blocks.
        let blocks = partition_blocks(instructions, &range, &leaders)?;

        // Step 3: Fill in predecessors from successors.
        let blocks = fill_predecessors(blocks);

        Ok(Cfg { blocks })
    }

    /// Build a CFG for each procedure in the program.
    pub fn build_per_procedure(program: &Program) -> Result<Vec<(u32, Cfg)>, CfgError> {
        let mut cfgs = Vec::new();
        for proc in &program.procedures {
            let cfg = Cfg::build(&program.instructions, proc.instr_start..proc.instr_end)?;
            cfgs.push((proc.entry_pc, cfg));
        }
        Ok(cfgs)
    }

    /// Return block PCs in reverse postorder (useful for dataflow analysis).
    pub fn reverse_postorder(&self) -> Vec<u32> {
        if self.blocks.is_empty() {
            return Vec::new();
        }
        let entry = *self.blocks.keys().next().unwrap();
        let mut visited = BTreeSet::new();
        let mut order = Vec::new();
        self.dfs_postorder(entry, &mut visited, &mut order);
        order.reverse();
        order
    }

    fn dfs_postorder(&self, pc: u32, visited: &mut BTreeSet<u32>, order: &mut Vec<u32>) {
        if !visited.insert(pc) {
            return;
        }
        if let Some(block) = self.blocks.get(&pc) {
            for &succ in &block.successors {
                self.dfs_postorder(succ, visited, order);
            }
        }
        order.push(pc);
    }
}

/// Identify leader PCs — instructions that start a new basic block.
fn find_leaders(instructions: &[Instruction], range: &std::ops::Range<usize>) -> BTreeSet<u32> {
    let mut leaders = BTreeSet::new();

    // First instruction is always a leader.
    if range.start < instructions.len() {
        leaders.insert(instructions[range.start].pc);
    }

    // Collect valid PCs for target validation.
    let valid_pcs: BTreeSet<u32> = instructions[range.clone()].iter().map(|i| i.pc).collect();

    for idx in range.clone() {
        let instr = &instructions[idx];
        match instr.op {
            // Unconditional jump: target is a leader.
            Opcode::Jmp => {
                if let Operand::Imm24(target) = instr.operand {
                    if valid_pcs.contains(&target) {
                        leaders.insert(target);
                    }
                }
            }
            // Conditional branches: target and fallthrough are both leaders.
            Opcode::Jz | Opcode::Jnz => {
                if let Operand::Imm24(target) = instr.operand {
                    if valid_pcs.contains(&target) {
                        leaders.insert(target);
                    }
                }
                // Instruction after the branch is a leader (fallthrough).
                if idx + 1 < range.end {
                    leaders.insert(instructions[idx + 1].pc);
                }
            }
            // Call: instruction after call is a leader (return point).
            Opcode::Call | Opcode::Calln => {
                if idx + 1 < range.end {
                    leaders.insert(instructions[idx + 1].pc);
                }
            }
            // Return/Halt: next instruction (if any) is a leader.
            Opcode::Ret | Opcode::Halt => {
                if idx + 1 < range.end {
                    leaders.insert(instructions[idx + 1].pc);
                }
            }
            // Trap: similar to halt.
            Opcode::Trap => {
                if idx + 1 < range.end {
                    leaders.insert(instructions[idx + 1].pc);
                }
            }
            _ => {}
        }
    }
    leaders
}

/// Partition instructions into basic blocks based on leader set.
fn partition_blocks(
    instructions: &[Instruction],
    range: &std::ops::Range<usize>,
    leaders: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BasicBlock>, CfgError> {
    let mut blocks = BTreeMap::new();

    // Map from PC to instruction index for fast lookup.
    let pc_to_idx: HashMap<u32, usize> = instructions[range.clone()]
        .iter()
        .enumerate()
        .map(|(i, instr)| (instr.pc, range.start + i))
        .collect();

    // Walk through instructions, splitting at leaders.
    let mut block_start_idx = range.start;
    let mut block_start_pc = instructions[range.start].pc;

    for idx in range.start..range.end {
        let is_last = idx + 1 >= range.end;
        let next_is_leader = !is_last && leaders.contains(&instructions[idx + 1].pc);
        let is_terminating = matches!(
            instructions[idx].op,
            Opcode::Jmp
                | Opcode::Jz
                | Opcode::Jnz
                | Opcode::Ret
                | Opcode::Halt
                | Opcode::Trap
                | Opcode::Call
                | Opcode::Calln
        );

        if is_last || next_is_leader || is_terminating {
            let instr_range = block_start_idx..idx + 1;
            let last_instr = &instructions[idx];
            let fallthrough_pc = if !is_last {
                Some(instructions[idx + 1].pc)
            } else {
                None
            };

            let (terminator, successors) =
                block_terminator(last_instr, fallthrough_pc, &pc_to_idx)?;

            blocks.insert(
                block_start_pc,
                BasicBlock {
                    start_pc: block_start_pc,
                    instr_range,
                    terminator,
                    successors,
                    predecessors: Vec::new(),
                },
            );

            if !is_last {
                block_start_idx = idx + 1;
                block_start_pc = instructions[idx + 1].pc;
            }
        }
    }

    Ok(blocks)
}

/// Determine the terminator and successors for a block ending with `last_instr`.
fn block_terminator(
    last_instr: &Instruction,
    fallthrough_pc: Option<u32>,
    pc_to_idx: &HashMap<u32, usize>,
) -> Result<(Terminator, Vec<u32>), CfgError> {
    match last_instr.op {
        Opcode::Jmp => {
            if let Operand::Imm24(target) = last_instr.operand {
                if pc_to_idx.contains_key(&target) {
                    Ok((Terminator::Jump(target), vec![target]))
                } else {
                    // Jump to outside this procedure (e.g., entry point jump).
                    // Treat as a jump with no in-procedure successor.
                    Ok((Terminator::Jump(target), vec![]))
                }
            } else {
                Ok((Terminator::Halt, vec![]))
            }
        }
        Opcode::Jz | Opcode::Jnz => {
            if let Operand::Imm24(target) = last_instr.operand {
                let ft = fallthrough_pc.unwrap_or(0);
                let mut succs = Vec::new();
                if pc_to_idx.contains_key(&target) {
                    succs.push(target);
                }
                if pc_to_idx.contains_key(&ft) {
                    succs.push(ft);
                }
                Ok((
                    Terminator::Branch {
                        target,
                        fallthrough: ft,
                    },
                    succs,
                ))
            } else {
                Ok((Terminator::Halt, vec![]))
            }
        }
        Opcode::Call | Opcode::Calln => {
            let target = match last_instr.operand {
                Operand::Imm24(addr) => addr,
                Operand::D8A24 { addr, .. } => addr,
                _ => 0,
            };
            let ft = fallthrough_pc.unwrap_or(0);
            // For CFG purposes within a procedure, calls are treated as
            // falling through (the callee returns to the next instruction).
            let mut succs = Vec::new();
            if let Some(ft_pc) = fallthrough_pc {
                if pc_to_idx.contains_key(&ft_pc) {
                    succs.push(ft_pc);
                }
            }
            Ok((
                Terminator::Call {
                    target,
                    fallthrough: ft,
                },
                succs,
            ))
        }
        Opcode::Ret => Ok((Terminator::Return, vec![])),
        Opcode::Halt | Opcode::Trap => Ok((Terminator::Halt, vec![])),
        _ => {
            // Non-terminating instruction at end of block — falls through.
            if let Some(ft_pc) = fallthrough_pc {
                if pc_to_idx.contains_key(&ft_pc) {
                    Ok((Terminator::Fallthrough, vec![ft_pc]))
                } else {
                    Ok((Terminator::Fallthrough, vec![]))
                }
            } else {
                // Last instruction in the range with no fallthrough.
                Ok((Terminator::Fallthrough, vec![]))
            }
        }
    }
}

/// Fill in predecessor lists from successor lists.
fn fill_predecessors(mut blocks: BTreeMap<u32, BasicBlock>) -> BTreeMap<u32, BasicBlock> {
    // Collect edges: (successor_pc, predecessor_pc).
    let edges: Vec<(u32, u32)> = blocks
        .values()
        .flat_map(|b| b.successors.iter().map(move |&s| (s, b.start_pc)))
        .collect();

    for (succ, pred) in edges {
        if let Some(block) = blocks.get_mut(&succ) {
            block.predecessors.push(pred);
        }
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use pcode_model::{Instruction, Opcode, Operand};

    fn instr(pc: u32, op: Opcode, operand: Operand) -> Instruction {
        Instruction { pc, op, operand }
    }

    #[test]
    fn single_block_no_branches() {
        let instrs = vec![
            instr(0, Opcode::Push, Operand::Imm24(42)),
            instr(4, Opcode::Push, Operand::Imm24(10)),
            instr(8, Opcode::Add, Operand::None),
            instr(9, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..4).unwrap();
        assert_eq!(cfg.blocks.len(), 1);
        let block = cfg.blocks.get(&0).unwrap();
        assert_eq!(block.terminator, Terminator::Halt);
        assert!(block.successors.is_empty());
    }

    #[test]
    fn unconditional_jump() {
        // 0: jmp 8
        // 4: push 1 (dead code, but still forms a block)
        // 8: halt
        let instrs = vec![
            instr(0, Opcode::Jmp, Operand::Imm24(8)),
            instr(4, Opcode::Push, Operand::Imm24(1)),
            instr(8, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..3).unwrap();
        // 3 blocks: [0: jmp], [4: push (dead)], [8: halt]
        assert_eq!(cfg.blocks.len(), 3);

        let b0 = cfg.blocks.get(&0).unwrap();
        assert_eq!(b0.terminator, Terminator::Jump(8));
        assert_eq!(b0.successors, vec![8]);

        let b8 = cfg.blocks.get(&8).unwrap();
        assert!(b8.predecessors.contains(&0));
        // Dead block at 4 falls through to 8 but has no predecessors itself.
        assert!(b8.predecessors.contains(&4));
    }

    #[test]
    fn conditional_branch() {
        // 0: push 0
        // 4: jz 12
        // 8: push 1
        // 12: halt
        let instrs = vec![
            instr(0, Opcode::Push, Operand::Imm24(0)),
            instr(4, Opcode::Jz, Operand::Imm24(12)),
            instr(8, Opcode::Push, Operand::Imm24(1)),
            instr(12, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..4).unwrap();

        // Block starting at 0 should contain push+jz
        let b0 = cfg.blocks.get(&0).unwrap();
        assert_eq!(
            b0.terminator,
            Terminator::Branch {
                target: 12,
                fallthrough: 8
            }
        );
        assert!(b0.successors.contains(&8));
        assert!(b0.successors.contains(&12));

        // Block at 12 should have predecessor 0 (jump) and 8 (fallthrough)
        let b12 = cfg.blocks.get(&12).unwrap();
        assert!(b12.predecessors.contains(&0));
    }

    #[test]
    fn while_loop_cfg() {
        // Simulates: while (cond) { body }
        // 0: push X         (loop header: load condition)
        // 4: jz 16          (exit loop if false)
        // 8: push 1         (body)
        // 12: jmp 0         (back edge)
        // 16: halt           (after loop)
        let instrs = vec![
            instr(0, Opcode::Push, Operand::Imm24(1)),
            instr(4, Opcode::Jz, Operand::Imm24(16)),
            instr(8, Opcode::Push, Operand::Imm24(1)),
            instr(12, Opcode::Jmp, Operand::Imm24(0)),
            instr(16, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..5).unwrap();

        // Loop header block (0) should have two predecessors: entry and back-edge from 8
        let b0 = cfg.blocks.get(&0).unwrap();
        assert!(b0.predecessors.contains(&8));

        // Body block (8) should jump back to 0
        let b8 = cfg.blocks.get(&8).unwrap();
        assert_eq!(b8.terminator, Terminator::Jump(0));
        assert!(b8.successors.contains(&0));
    }

    #[test]
    fn reverse_postorder_simple() {
        // 0: jz 8
        // 4: halt
        // 8: halt
        let instrs = vec![
            instr(0, Opcode::Jz, Operand::Imm24(8)),
            instr(4, Opcode::Halt, Operand::None),
            instr(8, Opcode::Halt, Operand::None),
        ];
        let cfg = Cfg::build(&instrs, 0..3).unwrap();
        let rpo = cfg.reverse_postorder();
        // Entry block should come first in RPO.
        assert_eq!(rpo[0], 0);
    }

    #[test]
    fn call_creates_fallthrough_block() {
        // 0: call 8
        // 4: halt
        // 8: ret 0
        let instrs = vec![
            instr(0, Opcode::Call, Operand::Imm24(8)),
            instr(4, Opcode::Halt, Operand::None),
            instr(8, Opcode::Ret, Operand::Imm8(0)),
        ];
        let cfg = Cfg::build(&instrs, 0..3).unwrap();
        // Call block should fall through to the next instruction.
        let b0 = cfg.blocks.get(&0).unwrap();
        assert!(matches!(b0.terminator, Terminator::Call { .. }));
        assert!(b0.successors.contains(&4));
    }
}
