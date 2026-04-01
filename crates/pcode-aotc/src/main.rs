//! pc-aotc: AOT compiler translating p-code bytecode into COR24 native assembly.

fn main() {
    println!("pc-aotc v{}", env!("CARGO_PKG_VERSION"));
    println!("Usage: pc-aotc <input.p24> -o <output.s>");
}
