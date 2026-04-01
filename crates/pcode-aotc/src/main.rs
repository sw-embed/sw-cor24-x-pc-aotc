//! pc-aotc: AOT compiler translating p-code bytecode into COR24 native assembly.
//!
//! Usage: pc-aotc <input.p24> [-o <output.s>]

use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "-h" || args[1] == "--help" {
        eprintln!("pc-aotc v{}", env!("CARGO_PKG_VERSION"));
        eprintln!("Usage: pc-aotc <input.p24> [-o <output.s>]");
        eprintln!();
        eprintln!("AOT compile p-code bytecode to COR24 native assembly.");
        process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    let input_path = PathBuf::from(&args[1]);
    let output_path = parse_output_path(&args, &input_path);

    // Read input
    let binary = match std::fs::read(&input_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", input_path.display());
            process::exit(1);
        }
    };

    // Decode
    let program = match pcode_model::decode_program(&binary) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: decode failed: {e}");
            process::exit(1);
        }
    };

    let source_name = input_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into());

    // Emit assembly
    let assembly = match cor24_emit_asm::emit_program(&program, &source_name) {
        Ok(asm) => asm,
        Err(e) => {
            eprintln!("error: emit failed: {e}");
            process::exit(1);
        }
    };

    // Write output
    match std::fs::write(&output_path, &assembly) {
        Ok(()) => {
            eprintln!(
                "pc-aotc: {} -> {} ({} instructions, {} bytes asm)",
                input_path.display(),
                output_path.display(),
                program.instructions.len(),
                assembly.len(),
            );
        }
        Err(e) => {
            eprintln!("error: cannot write {}: {e}", output_path.display());
            process::exit(1);
        }
    }
}

fn parse_output_path(args: &[String], input_path: &Path) -> PathBuf {
    // Look for -o <path>
    for i in 2..args.len() {
        if args[i] == "-o" && i + 1 < args.len() {
            return PathBuf::from(&args[i + 1]);
        }
    }
    // Default: replace extension with .s
    input_path.with_extension("s")
}
