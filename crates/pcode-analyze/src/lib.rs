//! CFG formation, stack-depth analysis, and validation for p-code.
//!
//! This crate provides control flow graph construction and stack-depth
//! propagation for decoded p-code programs. The CFG is used to validate
//! stack consistency at merge points and will support future optimizations.

mod cfg;
mod stack_depth;

pub use cfg::{BasicBlock, Cfg, CfgError, Terminator};
pub use stack_depth::{propagate_stack_depth, StackDepthError};
