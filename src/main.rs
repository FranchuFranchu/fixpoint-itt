#![feature(box_patterns, debug_closure_helpers)]

pub mod coherence;
pub mod lambda;
pub mod run;
pub mod tree;

fn main() {
    let code = std::fs::read_to_string(std::env::args().skip(1).next().unwrap()).unwrap();
    let mut book = match tree::TreeParser::new(&code).parse_book() {
        Ok(o) => o,
        Err(e) => todo!("{}", e),
    };
    book.root.normal();
    book.root.root.resolve_vars(&mut book.root.vars);
    println!("{}", book.root.display());
    println!("Is coherent? {}", book.root.root.is_coherent());
}
