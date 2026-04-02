//! Integration tests: build CFGs from real .p24 files and validate stack depth.

use pcode_analyze::{propagate_stack_depth, Cfg};
use pcode_model::decode_program;
use std::path::Path;

fn load_program(name: &str) -> pcode_model::Program {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/p24")
        .join(name);
    let bin =
        std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    decode_program(&bin).unwrap()
}

#[test]
fn arithmetic_cfg() {
    let prog = load_program("arithmetic.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    assert!(!cfgs.is_empty());
    for (_, cfg) in &cfgs {
        assert!(!cfg.blocks.is_empty());
    }
}

#[test]
fn if_else_cfg_has_branches() {
    let prog = load_program("if_else.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    // At least one procedure should have branching (multiple blocks).
    let max_blocks = cfgs
        .iter()
        .map(|(_, cfg)| cfg.blocks.len())
        .max()
        .unwrap_or(0);
    assert!(
        max_blocks >= 3,
        "if_else should have a procedure with at least 3 basic blocks, max was {max_blocks}",
    );
}

#[test]
fn while_loop_cfg_has_back_edge() {
    let prog = load_program("while_loop.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    // At least one procedure should have a back-edge (loop).
    let has_back_edge = cfgs.iter().any(|(_, cfg)| {
        cfg.blocks
            .values()
            .any(|b| b.successors.iter().any(|&s| s < b.start_pc))
    });
    assert!(
        has_back_edge,
        "while_loop should have at least one back-edge"
    );
}

#[test]
fn recursion_has_multiple_procedures() {
    let prog = load_program("recursion.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    assert!(
        cfgs.len() >= 2,
        "recursion should have at least 2 procedures, got {}",
        cfgs.len()
    );
}

#[test]
fn all_programs_stack_depth_consistent() {
    let names = [
        "arithmetic.p24",
        "if_else.p24",
        "while_loop.p24",
        "locals.p24",
        "globals.p24",
        "procedure_call.p24",
        "recursion.p24",
        "nested_calls.p24",
        "nested_loops.p24",
        "complex_if.p24",
        "for_loops.p24",
    ];
    for name in &names {
        let prog = load_program(name);
        let cfgs = Cfg::build_per_procedure(&prog).unwrap();
        for (entry_pc, cfg) in &cfgs {
            let result = propagate_stack_depth(cfg, &prog.instructions, 0);
            assert!(
                result.is_ok(),
                "{name}: stack depth error in procedure at 0x{entry_pc:04X}: {}",
                result.unwrap_err()
            );
        }
    }
}

#[test]
fn cfg_reverse_postorder_visits_all_reachable() {
    let prog = load_program("if_else.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    for (_, cfg) in &cfgs {
        let rpo = cfg.reverse_postorder();
        assert!(!rpo.is_empty() || cfg.blocks.is_empty());
    }
}

#[test]
fn nested_calls_multiple_procedures() {
    let prog = load_program("nested_calls.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    assert!(
        cfgs.len() >= 2,
        "nested_calls should have multiple procedures"
    );
}

#[test]
fn procedure_call_cfg() {
    let prog = load_program("procedure_call.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    // Should have at least 2 procedures (main + called procedure).
    assert!(cfgs.len() >= 2);
    // At least one CFG should have a Call terminator.
    let has_call = cfgs.iter().any(|(_, cfg)| {
        cfg.blocks
            .values()
            .any(|b| matches!(b.terminator, pcode_analyze::Terminator::Call { .. }))
    });
    assert!(
        has_call,
        "procedure_call should have at least one Call terminator"
    );
}

#[test]
fn nested_loops_has_multiple_back_edges() {
    let prog = load_program("nested_loops.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    // Nested loops should have at least 2 back-edges across all procedures.
    let back_edge_count: usize = cfgs
        .iter()
        .map(|(_, cfg)| {
            cfg.blocks
                .values()
                .filter(|b| b.successors.iter().any(|&s| s < b.start_pc))
                .count()
        })
        .sum();
    assert!(
        back_edge_count >= 2,
        "nested_loops should have at least 2 back-edges, got {back_edge_count}"
    );
}

#[test]
fn complex_if_has_many_blocks() {
    let prog = load_program("complex_if.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    let max_blocks = cfgs
        .iter()
        .map(|(_, cfg)| cfg.blocks.len())
        .max()
        .unwrap_or(0);
    assert!(
        max_blocks >= 5,
        "complex_if should have a procedure with many basic blocks, max was {max_blocks}"
    );
}

#[test]
fn for_loops_cfg() {
    let prog = load_program("for_loops.p24");
    let cfgs = Cfg::build_per_procedure(&prog).unwrap();
    // Should have back-edges from for loops.
    let has_back_edge = cfgs.iter().any(|(_, cfg)| {
        cfg.blocks
            .values()
            .any(|b| b.successors.iter().any(|&s| s < b.start_pc))
    });
    assert!(has_back_edge, "for_loops should have back-edges");
}
