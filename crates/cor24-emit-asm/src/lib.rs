//! COR24 assembly emission from decoded p-code programs.
//!
//! Translates p-code instructions into COR24 native assembly text (.s files).
//! Uses a memory-based eval stack via the hardware stack pointer (sp/r4).
//!
//! ## Register usage
//!
//! | Register | Alias | Purpose                        |
//! |----------|-------|--------------------------------|
//! | r0       |       | Scratch / operand              |
//! | r1       |       | Scratch / operand              |
//! | r2       |       | Scratch / jump target          |
//! | r3       | fp    | Frame pointer                  |
//! | r4       | sp    | Eval stack pointer (grows down)|
//! | r5       | z/c   | Zero register / condition flag |

mod emit;

pub use emit::{emit_program, EmitError};
